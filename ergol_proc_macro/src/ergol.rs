use proc_macro::TokenStream;

use syn::export::TokenStream2;
use syn::parse::{Parse, ParseStream};
use syn::{parenthesized, parse_macro_input, token, DeriveInput, Field, FieldsNamed, Ident};

use quote::{format_ident, quote};

/// Generates the token stream for an entity.
pub fn generate(mut input: DeriveInput) -> TokenStream {
    let mut fields = match &mut input.data {
        syn::Data::Struct(syn::DataStruct { fields, .. }) => match fields {
            syn::Fields::Named(fields) => fields,
            _ => panic!("Expecting named fields"),
        },
        _ => panic!("Expecting named fields"),
    };

    let to_many_to_many = fix_many_to_many_fields(&input.ident, &fields);

    let clone = fields.named.clone();
    let clone2 = fields.named.clone();
    let many_to_many_fields = clone
        .iter()
        .filter(|field| {
            field.attrs.iter().any(|attr| {
                attr.path.get_ident().map(Ident::to_string) == Some(String::from("many_to_many"))
            })
        })
        .collect::<Vec<_>>();

    fields.named.clear();

    for field in clone2 {
        let should_add = field.attrs.iter().all(|attr| {
            attr.path.get_ident().map(Ident::to_string) != Some(String::from("many_to_many"))
        });
        if should_add {
            fields.named.push(field);
        }
    }

    let to_one_to_one = fix_one_to_one_fields(&input.ident, &mut fields);
    let to_many_to_one = fix_many_to_one_fields(&input.ident, &mut fields);

    let (field_id, other_fields) = find_id(fields).unwrap();
    let unique_fields = find_unique(fields);

    let to_table = to_table(
        &input.ident,
        &field_id,
        &other_fields,
        &many_to_many_fields.as_slice(),
    );
    let to_impl = to_impl(&input.ident, &field_id, &other_fields);
    let to_unique = to_unique(&input.ident, &field_id, &unique_fields);

    for field in &mut fields.named {
        field.attrs = field
            .attrs
            .clone()
            .into_iter()
            .filter(|attr| {
                let s = attr.path.get_ident().map(Ident::to_string);
                s != Some(String::from("id"))
                    && s != Some(String::from("unique"))
                    && s != Some(String::from("one_to_one"))
                    && s != Some(String::from("many_to_one"))
                    && s != Some(String::from("many_to_many"))
            })
            .collect();
    }

    let q = quote! {
        #[derive(Debug)]
        #input
        #to_impl
        #to_unique
        #to_table
        #to_one_to_one
        #to_many_to_one
        #to_many_to_many
    };

    q.into()
}

/// Finds the field marked as id in a fieldsnamed.
pub fn find_id(fields: &FieldsNamed) -> Option<(&Field, Vec<&Field>)> {
    let mut other_fields = vec![];
    let mut id = None;

    'outer: for field in fields.named.iter() {
        for attr in &field.attrs {
            match attr.path.get_ident() {
                Some(i) if &i.to_string() == "id" && id.is_some() => return None,
                Some(i) if &i.to_string() == "id" => {
                    id = Some(field);
                    continue 'outer;
                }
                _ => (),
            }
        }
        other_fields.push(field);
    }

    match id {
        Some(id) => Some((id, other_fields)),
        _ => None,
    }
}

/// Finds all the fields marked as unique
pub fn find_unique(fields: &FieldsNamed) -> Vec<&Field> {
    let mut output = vec![];

    'outer: for field in fields.named.iter() {
        for attr in &field.attrs {
            match attr.path.get_ident() {
                Some(i) if &i.to_string() == "unique" => {
                    output.push(field);
                    continue 'outer;
                }
                _ => (),
            }
        }
    }

    output
}

/// Generates the ToTable implementation.
pub fn to_table(
    name: &Ident,
    id: &Field,
    other_fields: &[&Field],
    many_to_many_fields: &[&Field],
) -> TokenStream2 {
    use case::CaseExt;

    let name_snake = format_ident!("{}", name.to_string().to_snake());
    let table_name = format_ident!("{}s", name_snake);
    let id_ident = id.ident.as_ref().unwrap();
    let id_name = format_ident!("{}", id_ident.to_string());

    let row = quote!(ergol::tokio_postgres::Row);

    let mut create_table = vec![];
    create_table.push(format!("CREATE TABLE {} (\n", table_name));
    create_table.push(format!("    {} SERIAL PRIMARY KEY,\n", id_name));

    let mut field_types = vec![];
    let mut field_names = vec![];
    let field_indices = (1..other_fields.len() + 1).map(syn::Index::from);

    for field in other_fields {
        create_table.push(format!(
            "    {} {{}},\n",
            field.ident.as_ref().unwrap().to_string()
        ));

        field_types.push(&field.ty);
        field_names.push(&field.ident);
    }

    let mut create_table = create_table.join("");
    create_table.pop();
    create_table.pop();
    create_table.push_str("\n);");

    let mut create_tables = vec![];
    for field in many_to_many_fields {
        let mut new = vec![];
        new.push(format!(
            "CREATE TABLE {}_{}_join (\n",
            table_name,
            format_ident!("{}", field.ident.as_ref().unwrap())
        ));

        new.push(format!("    id SERIAL PRIMARY KEY,\n"));

        new.push(format!(
            "    {}_id INT NOT NULL REFERENCES {},\n",
            table_name, table_name,
        ));

        let ty = &field.ty;
        let name = format!("{}s", quote! {#ty}.to_string().to_snake());

        new.push(format!(
            "    {}_id INT NOT NULL REFERENCES {},\n",
            field.ident.as_ref().unwrap(),
            name,
        ));

        let mut new = new.join("");
        new.pop();
        new.pop();
        new.push_str("\n);");

        create_tables.push(new);
    }

    let mut drop_tables = vec![format!("DROP TABLE {} CASCADE;", table_name)];

    for field in many_to_many_fields {
        drop_tables.push(format!(
            "DROP TABLE {}_{}_join CASCADE;",
            table_name,
            format_ident!("{}", field.ident.as_ref().unwrap())
        ));
    }

    let field_names = field_names.iter();
    let field_names2 = field_names.clone();

    quote! {
        impl ergol::ToTable for #name {
            fn from_row(row: #row) -> Self {
                #name {
                    #id_ident: row.get(0),
                    #(
                        #field_names: row.get(#field_indices),
                    )*
                }
            }

            fn table_name() -> &'static str {
                stringify!(#table_name)
            }

            fn id_name() -> &'static str {
                stringify!(#id_name)
            }

            fn id(&self) -> i32 {
                self.#id_ident
            }

            fn create_table() -> ergol::query::CreateTable {
                ergol::query::CreateTable(vec![
                    format!(#create_table, #(<#field_types as Pg>::ty(), )*),
                    #(
                        String::from(#create_tables),
                    )*
                ])
            }

            fn drop_table() -> ergol::query::DropTable {
                ergol::query::DropTable(vec![
                    #(
                        #drop_tables.to_owned(),
                    )*
                ])
            }

            fn select() -> ergol::query::Select<Self> {
                ergol::query::Select::new()
            }
        }

        /// Module that contains the columns of the table.
        pub mod #name_snake {
            #(

                /// Module that contains the helpers for the column.
                pub mod #field_names2 {

                    /// Keeps only the results for which the column equals the value passed as
                    /// parameter.
                    pub fn eq<T: ergol::tokio_postgres::types::ToSql + Sync + Send + 'static>(t: T) -> ergol::query::Filter {
                        ergol::query::Filter {
                            column: stringify!(#field_names2),
                            value: Box::new(t),
                            operator: ergol::query::Operator::Eq,
                        }
                    }

                    /// Keeps only the results for which the column is different from the value
                    /// passed as parameter.
                    pub fn neq<T: ergol::tokio_postgres::types::ToSql + Sync + Send + 'static>(t: T) -> ergol::query::Filter {
                        ergol::query::Filter {
                            column: stringify!(#field_names2),
                            value: Box::new(t),
                            operator: ergol::query::Operator::Neq,
                        }
                    }

                    /// Keeps only the results for which the column is lesser or equals the value
                    /// passed as parameter.
                    pub fn leq<T: ergol::tokio_postgres::types::ToSql + Sync + Send + 'static>(t: T) -> ergol::query::Filter {
                        ergol::query::Filter {
                            column: stringify!(#field_names2),
                            value: Box::new(t),
                            operator: ergol::query::Operator::Leq,
                        }
                    }

                    /// Keeps only the results for which the column is greater or equals the value
                    /// passed as parameter.
                    pub fn geq<T: ergol::tokio_postgres::types::ToSql + Sync + Send + 'static>(t: T) -> ergol::query::Filter {
                        ergol::query::Filter {
                            column: stringify!(#field_names2),
                            value: Box::new(t),
                            operator: ergol::query::Operator::Geq,
                        }
                    }

                    /// Keeps only the results for which the column is lesser than the value passed
                    /// as parameter.
                    pub fn lt<T: ergol::tokio_postgres::types::ToSql + Sync + Send + 'static>(t: T) -> ergol::query::Filter {
                        ergol::query::Filter {
                            column: stringify!(#field_names2),
                            value: Box::new(t),
                            operator: ergol::query::Operator::Lt,
                        }
                    }

                    /// Keeps only the results for which the column is greater than the value passed
                    /// as parameter.
                    pub fn gt<T: ergol::tokio_postgres::types::ToSql + Sync + Send + 'static>(t: T) -> ergol::query::Filter {
                        ergol::query::Filter {
                            column: stringify!(#field_names2),
                            value: Box::new(t),
                            operator: ergol::query::Operator::Gt,
                        }
                    }
                }
            )*
        }
    }
}

/// Generates some helper functions for the type.
pub fn to_impl(name: &Ident, id_field: &Field, other_fields: &[&Field]) -> TokenStream2 {
    let id_name = id_field.ident.as_ref().unwrap();

    use case::CaseExt;
    let table_name = format_ident!("{}s", name.to_string().to_snake());
    let db = quote! { ergol::tokio_postgres::Client };
    let error = quote! { ergol::tokio_postgres::Error };

    let without_id = format_ident!("{}WithoutId", name);

    let field_comment = other_fields
        .iter()
        .map(|field| {
            for attr in &field.attrs {
                if attr.path.get_ident().map(|x| x.to_string()) == Some(String::from("doc")) {
                    return Some(attr);
                }
            }
            None
        })
        .collect::<Vec<_>>();

    let names = other_fields.iter().map(|field| &field.ident);
    let names2 = names.clone();
    let names3 = names.clone();
    let names4 = names.clone();
    let names5 = names.clone();

    let names_as_strings = names
        .clone()
        .map(|x| x.as_ref().unwrap().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let original_types = other_fields.iter().map(|field| &field.ty);

    let intos = other_fields.iter().map(|field| {
        let ty = &field.ty;
        quote! { Into<#ty> }
    });

    let types = other_fields
        .iter()
        .enumerate()
        .map(|(id, _)| format_ident!("T{}", id));

    let types2 = types.clone();

    let dollars = (1..other_fields.len() + 1)
        .map(|x| format!("${}", x))
        .collect::<Vec<_>>()
        .join(", ");

    let names_and_dollars = names
        .clone()
        .enumerate()
        .map(|(i, name)| format!("{} = ${}", name.as_ref().unwrap(), i + 1))
        .collect::<Vec<_>>()
        .join(", ");

    let last_dollar = format!("${}", other_fields.len() + 1);

    let insert_query = format!(
        "INSERT INTO {}({}) VALUES({}) RETURNING *;",
        table_name, names_as_strings, dollars,
    );

    let update_query = format!(
        "UPDATE {} SET {} WHERE {} = {};",
        table_name,
        names_and_dollars,
        id_field.ident.as_ref().unwrap(),
        last_dollar
    );

    let delete_query = format!(
        "DELETE FROM {} WHERE {} = $1;",
        table_name,
        id_field.ident.as_ref().unwrap(),
    );

    let without_id_doc = format!("{} is like {}, but without the id.", without_id, name);

    quote! {
        #[doc=#without_id_doc]
        ///
        /// It is used to insert a new value in the database without specifiying the id, which will
        /// be automatically generated by the database system.
        pub struct #without_id {
            #(
                #field_comment
                #names3: #original_types,
            )*
        }

        impl #without_id {
            /// Inserts the element into the database, returning the real element with its id.
            pub async fn save(self, db: &#db) -> Result<#name, #error> {
                let row = db.query_one(#insert_query, &[ #( &self.#names4, )* ]).await?;
                Ok(<#name as ergol::ToTable>::from_row(row))
            }
        }

        impl #name {
            /// Creates a new element, without id, and with parameters specified in the same order
            /// as in the struct.
            ///
            /// This function tries to convert its inputs to the type in the struct, so you can
            /// easily manage strings for example.
            pub fn create<#(#types: #intos,)*>(#(#names: #types2, )*) -> #without_id {
                #without_id {
                    #(
                        #names2: #names2.into(),
                    )*
                }
            }

            /// Updates every field of the element in the database.
            pub async fn save(&self, db: &#db) -> Result<(), #error> {
                db.query(#update_query, &[ #( &self.#names5, )* &self.#id_name ]).await?;
                Ok(())
            }

            /// Deletes self from the database.
            pub async fn delete(self, db: &#db) -> Result<(), #error> {
                db.query(#delete_query, &[&self.id()]).await?;
                Ok(())
            }
        }
    }
}

/// Generates the getters for the unique fields.
pub fn to_unique(name: &Ident, id_field: &Field, other_fields: &[&Field]) -> TokenStream2 {
    use case::CaseExt;
    let table_name = format_ident!("{}s", name.to_string().to_snake());

    let db = quote! { ergol::tokio_postgres::Client };
    let error = quote! { ergol::tokio_postgres::Error };

    let fields = &[id_field];

    let fields = fields.iter().chain(other_fields.iter());

    let getters = fields
        .clone()
        .map(|field| format_ident!("get_by_{}", field.ident.as_ref().unwrap()));

    let types = fields.clone().map(|field| &field.ty);

    let queries = fields.clone().map(|field| {
        format!(
            "SELECT * FROM {} WHERE {} = $1",
            table_name,
            field.ident.as_ref().unwrap()
        )
    });

    let doc = fields.clone().map(|g| {
        format!(
            "Retrieves the {} based on its {} attribute, which is specified as unique in the database.",
            name,
            g.ident.as_ref().unwrap()
        )
    });

    quote! {
        impl #name {
            #(
                #[doc=#doc]
                pub async fn #getters<T: Into<#types>>(attr: T, db: &#db) -> Result<Option<#name>, #error> {
                    let mut rows = db.query(#queries, &[&attr.into()]).await?;
                    Ok(rows.pop().map(<#name as ToTable>::from_row))
                }
            )*
        }
    }
}

/// Struct to help parse the map_by attribute.
struct MappedBy {
    pub paren_token: token::Paren,
    pub name: Ident,
}

impl Parse for MappedBy {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(MappedBy {
            paren_token: parenthesized!(content in input),
            name: content.parse()?,
        })
    }
}

/// Changes the types of one to one fields.
pub fn fix_one_to_one_fields(name: &Ident, fields: &mut FieldsNamed) -> TokenStream2 {
    use case::CaseExt;
    let table_name = format_ident!("{}s", name.to_string().to_snake());
    let db = quote! { ergol::tokio_postgres::Client };
    let error = quote! { ergol::tokio_postgres::Error };

    let fields_clone: FieldsNamed = fields.clone();

    let mut fields_to_fix = fields.named.iter_mut().filter(|field| {
        field.attrs.iter().any(|attr| {
            attr.path.get_ident().map(Ident::to_string) == Some(String::from("one_to_one"))
        })
    });

    let fields_clone = fields_clone.named.iter().filter(|field| {
        field.attrs.iter().any(|attr| {
            attr.path.get_ident().map(Ident::to_string) == Some(String::from("one_to_one"))
        })
    });

    let idents = fields_clone.clone().map(|x| x.ident.as_ref().unwrap());
    let types = fields_clone.clone().map(|x| &x.ty);

    let tokens = fields_clone
        .clone()
        .map(|x| {
            x.attrs
                .iter()
                .find(|attr| {
                    attr.path.get_ident().map(Ident::to_string) == Some(String::from("one_to_one"))
                })
                .unwrap()
        })
        .map(|x| Into::<TokenStream>::into(x.tokens.clone()))
        .map(|tokens| {
            let m = parse_macro_input!(tokens as MappedBy);
            let name = m.name;
            let q = quote! { #name };
            q.into()
        })
        .map(Into::<TokenStream2>::into);

    let query = fields_clone.clone().map(|field| {
        format!(
            "SELECT * FROM {} WHERE {} = $1",
            table_name,
            field.ident.as_ref().unwrap()
        )
    });

    let idents_doc = idents.clone().zip(types.clone()).map(|(ident, ty)| {
        format!(
            "Helper function to retrieve the {} from the {}",
            ident.to_string().to_snake(),
            quote! { #ty }.to_string().to_snake(),
        )
    });

    let tokens_doc = tokens.clone().zip(types.clone()).map(|(tokens, ty)| {
        format!(
            "Helper function to retrieve the {} from the {}.",
            quote! { #tokens }.to_string().to_snake(),
            quote! { #ty }.to_string().to_snake(),
        )
    });

    let q = quote! {
        #(
            impl #name {
                #[doc=#idents_doc]
                pub async fn #idents(&self, db: &#db) -> Result<#types, #error> {
                    Ok(self.#idents.fetch(db).await?)
                }
            }

            impl #types {
                #[doc=#tokens_doc]
                pub async fn #tokens(&self, db: &#db) -> Result<Option<#name>, #error> {
                    let mut rows = db.query(#query, &[&self.id]).await?;
                    Ok(rows.pop().map(#name::from_row))
                }
            }
        )*
    };

    for field in &mut fields_to_fix {
        let ty = &field.ty;
        field.ty = syn::Type::Verbatim(quote! { ergol::relation::OneToOne<#ty> });
    }

    q
}

/// Changes the types of many to one fields.
pub fn fix_many_to_one_fields(name: &Ident, fields: &mut FieldsNamed) -> TokenStream2 {
    use case::CaseExt;
    let table_name = format_ident!("{}s", name.to_string().to_snake());
    let db = quote! { ergol::tokio_postgres::Client };
    let error = quote! { ergol::tokio_postgres::Error };

    let fields_clone: FieldsNamed = fields.clone();

    let mut fields_to_fix = fields.named.iter_mut().filter(|field| {
        field.attrs.iter().any(|attr| {
            attr.path.get_ident().map(Ident::to_string) == Some(String::from("many_to_one"))
        })
    });

    let fields_clone = fields_clone.named.iter().filter(|field| {
        field.attrs.iter().any(|attr| {
            attr.path.get_ident().map(Ident::to_string) == Some(String::from("many_to_one"))
        })
    });

    let idents = fields_clone.clone().map(|x| x.ident.as_ref().unwrap());
    let types = fields_clone.clone().map(|x| &x.ty);

    let tokens = fields_clone
        .clone()
        .map(|x| {
            x.attrs
                .iter()
                .find(|attr| {
                    attr.path.get_ident().map(Ident::to_string) == Some(String::from("many_to_one"))
                })
                .unwrap()
        })
        .map(|x| Into::<TokenStream>::into(x.tokens.clone()))
        .map(|tokens| {
            let m = parse_macro_input!(tokens as MappedBy);
            let name = m.name;
            let q = quote! { #name };
            q.into()
        })
        .map(Into::<TokenStream2>::into);

    let query = fields_clone.map(|field| {
        format!(
            "SELECT * FROM {} WHERE {} = $1",
            table_name,
            field.ident.as_ref().unwrap()
        )
    });

    let idents_doc = idents.clone().map(|ident| {
        format!(
            "Helper function to retrieve the {} from the {}.",
            ident.to_string().to_snake(),
            name.to_string().to_snake(),
        )
    });

    let tokens_doc = tokens.clone().zip(types.clone()).map(|(tokens, ty)| {
        format!(
            "Helper function to retrieve the {} from the {}.",
            quote! { #tokens }.to_string().to_snake(),
            quote! { #ty }.to_string().to_snake(),
        )
    });

    let q = quote! {
        #(
            impl #name {
                #[doc=#idents_doc]
                pub async fn #idents(&self, db: &#db) -> Result<#types, #error> {
                    Ok(self.#idents.fetch(db).await?)
                }
            }

            impl #types {
                #[doc=#tokens_doc]
                pub async fn #tokens(&self, db: &#db) -> Result<Vec<#name>, #error> {
                    let mut rows = db.query(#query, &[&self.id]).await?;
                    Ok(rows.into_iter().map(#name::from_row).collect::<Vec<_>>())
                }
            }
        )*
    };

    for field in &mut fields_to_fix {
        let ty = &field.ty;
        field.ty = syn::Type::Verbatim(quote! { ergol::relation::ManyToOne<#ty> });
    }

    q
}

/// Changes the types of many to many fields.
pub fn fix_many_to_many_fields(name: &Ident, fields: &FieldsNamed) -> TokenStream2 {
    use case::CaseExt;
    let table_name = format_ident!("{}s", name.to_string().to_snake());
    let db = quote! { ergol::tokio_postgres::Client };
    let error = quote! { ergol::tokio_postgres::Error };

    let fields_to_fix = fields.named.iter().filter(|field| {
        field.attrs.iter().any(|attr| {
            attr.path.get_ident().map(Ident::to_string) == Some(String::from("many_to_many"))
        })
    });

    let names = fields_to_fix.clone().map(|x| &x.ident);
    let add_names = fields_to_fix.clone().map(|x| {
        format_ident!("add_{}", {
            let mut p = x.ident.as_ref().unwrap().to_string();
            p.pop();
            p
        })
    });

    let delete_names = fields_to_fix.clone().map(|x| {
        format_ident!("remove_{}", {
            let mut p = x.ident.as_ref().unwrap().to_string();
            p.pop();
            p
        })
    });

    let insert_queries = fields_to_fix.clone().map(|x| {
        let y = format_ident!("{}_{}_join", table_name, x.ident.as_ref().unwrap()).to_string();
        format!(
            "INSERT INTO {}({}_id, {}_id) VALUES ($1, $2);",
            y,
            table_name,
            x.ident.as_ref().unwrap(),
        )
    });

    let delete_queries = fields_to_fix.clone().map(|x| {
        let y = format_ident!("{}_{}_join", table_name, x.ident.as_ref().unwrap()).to_string();
        format!(
            "DELETE FROM {} WHERE {}_id = $1 AND {}_id = $2 RETURNING id;",
            y,
            table_name,
            x.ident.as_ref().unwrap(),
        )
    });

    let types = fields_to_fix.clone().map(|x| &x.ty);
    let types_names = types
        .clone()
        .map(|x| format!("{}s", quote! {#x}.to_string().to_snake()));

    let tokens = fields_to_fix
        .clone()
        .map(|x| {
            x.attrs
                .iter()
                .find(|attr| {
                    attr.path.get_ident().map(Ident::to_string)
                        == Some(String::from("many_to_many"))
                })
                .unwrap()
        })
        .map(|x| Into::<TokenStream>::into(x.tokens.clone()))
        .map(|tokens| {
            let m = parse_macro_input!(tokens as MappedBy);
            let name = m.name;
            let q = quote! { #name };
            q.into()
        })
        .map(Into::<TokenStream2>::into);

    let add_tokens = fields_to_fix
        .clone()
        .map(|x| {
            x.attrs
                .iter()
                .find(|attr| {
                    attr.path.get_ident().map(Ident::to_string)
                        == Some(String::from("many_to_many"))
                })
                .unwrap()
        })
        .map(|x| Into::<TokenStream>::into(x.tokens.clone()))
        .map(|tokens| {
            let m = parse_macro_input!(tokens as MappedBy);
            let mut name = format!("add_{}", m.name.to_string());
            name.pop();
            let name = format_ident!("{}", name);
            let q = quote! { #name };
            q.into()
        })
        .map(Into::<TokenStream2>::into);

    let delete_tokens = fields_to_fix
        .clone()
        .map(|x| {
            x.attrs
                .iter()
                .find(|attr| {
                    attr.path.get_ident().map(Ident::to_string)
                        == Some(String::from("many_to_many"))
                })
                .unwrap()
        })
        .map(|x| Into::<TokenStream>::into(x.tokens.clone()))
        .map(|tokens| {
            let m = parse_macro_input!(tokens as MappedBy);
            let mut name = format!("remove_{}", m.name.to_string());
            name.pop();
            let name = format_ident!("{}", name);
            let q = quote! { #name };
            q.into()
        })
        .map(Into::<TokenStream2>::into);

    let select_queries = fields_to_fix.clone().zip(types_names).map(|(x, z)| {
        let y = format_ident!("{}_{}_join", table_name, x.ident.as_ref().unwrap()).to_string();
        format!(
            "SELECT {3}.* FROM {},{3} WHERE {}_id = $1 AND {3}.id = {}_id;",
            y,
            table_name,
            x.ident.as_ref().unwrap(),
            z,
        )
    });

    let query = fields_to_fix.map(|x| {
        let y = format_ident!("{}_{}_join", table_name, x.ident.as_ref().unwrap()).to_string();
        format!(
            "SELECT {}.* FROM {},{} WHERE {}_id = $1 AND {}_id = {}.id;",
            table_name,
            y,
            table_name,
            x.ident.as_ref().unwrap(),
            table_name,
            table_name,
        )
    });

    let q = quote! {

        #(
            impl #name {
                /// TODO fix doc
                pub async fn #add_names(&self, name: &#types, db: &#db) -> Result<(), #error> {
                    let rows = db.query(#insert_queries, &[&self.id, &name.id]).await?;
                    Ok(())
                }

                /// TODO fix doc
                pub async fn #delete_names(&self, name: &#types, db: &#db) -> Result<bool, #error> {
                    let rows = db.query(#delete_queries, &[&self.id, &name.id]).await?;
                    Ok(rows.len() > 0)
                }

                /// TODO fix doc
                pub async fn #names(&self, db: &#db) -> Result<Vec<#types>, #error> {
                    let rows = db.query(#select_queries, &[&self.id]).await?;
                    Ok(rows.into_iter().map(|x| #types::from_row(x)).collect::<Vec<_>>())
                }
            }

            impl #types {
                /// TODO fix doc
                pub async fn #tokens(&self, db: &#db) -> Result<Vec<#name>, #error> {
                    let mut rows = db.query(#query, &[&self.id]).await?;
                    Ok(rows.into_iter().map(|x| #name::from_row(x)).collect::<Vec<_>>())
                }

                /// TODO fix doc
                pub async fn #add_tokens(&self, other: &#name, db: &#db) -> Result<(), #error> {
                    db.query(#insert_queries, &[&other.id, &self.id]).await?;
                    Ok(())
                }

                /// TODO fix doc
                pub async fn #delete_tokens(&self, other: &#name, db: &#db) -> Result<bool, #error> {
                    let rows = db.query(#delete_queries, &[&other.id, &self.id]).await?;
                    Ok(rows.len() > 0)
                }
            }
        )*
    };

    q
}
