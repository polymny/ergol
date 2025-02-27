use std::fs::{create_dir_all, File};
use std::io::Write;
use std::str::FromStr;

use proc_macro::TokenStream;

use proc_macro2::TokenStream as TokenStream2;

use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{
    parenthesized, parse, parse_macro_input, token, Attribute, DeriveInput, Field, FieldsNamed,
    Ident, Token,
};

use quote::{format_ident, quote};

use ergol_core::{Column, Element, Table, Ty};

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
        .filter(|field| find_attribute(field, "many_to_many").is_some())
        .collect::<Vec<_>>();

    let (field_id, other_fields) = find_id(fields).unwrap();
    let json = to_json(&input.ident, &field_id, &other_fields);

    fields.named.clear();

    for field in clone2 {
        if find_attribute(&field, "many_to_many").is_none() {
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

    // Generate json representation of the table.
    create_dir_all("migrations/current").unwrap();
    let mut file = File::create(format!("migrations/current/{}.json", &input.ident)).unwrap();
    file.write_all(serde_json::to_string_pretty(&json).unwrap().as_bytes())
        .unwrap();

    match File::create(format!("migrations/.gitignore")) {
        Ok(mut f) => f.write_all(b"current\n").unwrap(),
        _ => (),
    };

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

/// Helper to find whether a field has a specific attribute.
pub fn find_attribute<'a>(field: &'a Field, attr: &str) -> Option<&'a Attribute> {
    field
        .attrs
        .iter()
        .find(|x| x.path.get_ident().map(Ident::to_string) == Some(String::from(attr)))
}

/// Generates the json.
pub fn to_json(name: &Ident, id: &Field, other_fields: &[&Field]) -> Vec<Element> {
    use case::CaseExt;

    let name_snake = format_ident!("{}", name.to_string().to_snake());
    let table_name = format_ident!("{}s", name_snake);
    let table_name_format = format!("{}", table_name);
    let id_ident = id.ident.as_ref().unwrap();
    let id_name = format_ident!("{}", id_ident.to_string());

    let mut output = vec![];
    let mut json = Table::new(&table_name_format);

    json.columns
        .push(Column::new(&format!("{}", id_name), Ty::Id, false));

    for field in other_fields {
        let ty = &field.ty;

        if let Some(attr) = find_attribute(field, "many_to_many") {
            let tokens = Into::<TokenStream>::into(attr.tokens.clone());
            let m = parse::<MappedBy>(tokens).unwrap();
            let extras = m.names.into_iter().skip(1).collect::<Vec<_>>();
            let mut table = Table::new(&format!(
                "{}_{}_join",
                table_name,
                format_ident!("{}", field.ident.as_ref().unwrap())
            ));

            // Primary key of table
            table.columns.push(Column::new("id", Ty::Id, false));

            // Id of the first link
            table.columns.push(Column::new(
                &format!("{}_id", table_name_format),
                Ty::Reference(table_name_format.clone()),
                false,
            ));

            // Id of the second link
            let name = format!("{}s", quote! {#ty}.to_string().to_snake());
            table.columns.push(Column::new(
                &format!("{}_id", field.ident.as_ref().unwrap()),
                Ty::Reference(name),
                false,
            ));

            // Extra info
            for extra in extras {
                let e = extra.to_string().to_snake();
                table
                    .columns
                    .push(Column::new(&e, Ty::from_str(&e).unwrap(), false));
            }

            output.push(Element::Table(table));
        } else if find_attribute(field, "one_to_one").is_some()
            || find_attribute(field, "many_to_one").is_some()
        {
            json.columns.push(Column::new(
                &format!("{}", field.ident.as_ref().unwrap()),
                Ty::Reference(format!("{}s", quote! { #ty }).to_snake()),
                false,
            ));
        } else {
            json.columns.push(Column::new(
                &format!("{}", field.ident.as_ref().unwrap()),
                Ty::from_str(&format!("{}", quote! { #ty })).unwrap(),
                find_attribute(field, "unique").is_some(),
            ));
        }
    }

    output.insert(0, Element::Table(json));
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
    create_table.push(format!("CREATE TABLE \"{}\" (\n", table_name));
    create_table.push(format!("    \"{}\" SERIAL PRIMARY KEY,\n", id_name));

    let mut field_types = vec![];
    let mut field_names = vec![];
    let field_indices = (1..other_fields.len() + 1).map(syn::Index::from);

    for field in other_fields {
        create_table.push(format!(
            "    \"{}\" {{}}{},\n",
            field.ident.as_ref().unwrap().to_string(),
            if find_attribute(field, "unique").is_some() {
                " UNIQUE"
            } else {
                ""
            }
        ));

        field_types.push(&field.ty);
        field_names.push(&field.ident);
    }

    let mut create_table = create_table.join("");
    create_table.pop();
    create_table.pop();
    create_table.push_str("\n);");

    let extra = many_to_many_fields
        .iter()
        .map(|x| find_attribute(x, "many_to_many").unwrap())
        .map(|x| Into::<TokenStream>::into(x.tokens.clone()))
        .map(|tokens| {
            let m = parse::<MappedBy>(tokens).unwrap();
            m.names.into_iter().skip(1).collect::<Vec<_>>()
        });

    let create_tables = many_to_many_fields
        .iter()
        .zip(extra)
        .map(|(field, extra)| {
            let mut new = vec![];
            new.push(format!(
                "CREATE TABLE \"{}_{}_join\" (\n",
                table_name,
                format_ident!("{}", field.ident.as_ref().unwrap())
            ));

            new.push(format!("    \"id\" SERIAL PRIMARY KEY,\n"));

            new.push(format!(
                "    \"{}_id\" INT NOT NULL REFERENCES \"{}\" ON DELETE CASCADE,\n",
                table_name, table_name,
            ));

            let ty = &field.ty;
            let name = format!("{}s", quote! {#ty}.to_string().to_snake());

            new.push(format!(
                "    \"{}_id\" INT NOT NULL REFERENCES \"{}\" ON DELETE CASCADE,\n",
                field.ident.as_ref().unwrap(),
                name,
            ));

            for extra in extra {
                let extra = extra.to_string().to_snake();
                new.push(format!("     \"{}\" {} NOT NULL,\n", extra, extra));
            }

            let mut new = new.join("");
            new.pop();
            new.pop();
            new.push_str("\n);");

            new
        })
        .collect::<Vec<_>>();

    let mut drop_tables = vec![format!("DROP TABLE \"{}\" CASCADE;", table_name)];

    for field in many_to_many_fields {
        drop_tables.push(format!(
            "DROP TABLE \"{}_{}_join\" CASCADE;",
            table_name,
            format_ident!("{}", field.ident.as_ref().unwrap())
        ));
    }

    let field_names = field_names.iter();
    let field_names2 = field_names.clone();

    let field_likes = field_names2
        .clone()
        .zip(field_types.clone())
        .map(|(x, y)| {
            if quote! { #y }.to_string() == "String" {
                quote! {
                    /// Construct a like query.
                    pub fn like<T: ergol::tokio_postgres::types::ToSql + Sync + Send + 'static>(t: T) -> ergol::query::Filter {
                        ergol::query::Filter::Binary {
                            column: stringify!(#x),
                            value: Box::new(t),
                            operator: ergol::query::Operator::Like,
                        }
                    }

                    /// Construct a similar to query.
                    pub fn similar_to<T: ergol::tokio_postgres::types::ToSql + Sync + Send + 'static>(t: T) -> ergol::query::Filter {
                        ergol::query::Filter::Binary {
                            column: stringify!(#x),
                            value: Box::new(t),
                            operator: ergol::query::Operator::SimilarTo,
                        }
                    }
                }
            } else {
                quote! {}
            }
        })
        .collect::<Vec<_>>();

    let tokens = quote! {
        impl ergol::ToTable for #name {
            fn from_row_with_offset(row: &#row, offset: usize) -> Self {
                #name {
                    #id_ident: row.get(offset),
                    #(
                        #field_names: row.get(offset + #field_indices),
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

            /// Module that contains the helpers for the column.
            pub mod #id_name {
                /// Keeps only the results for which the column equals the value passed as
                /// parameter.
                pub fn eq<T: ergol::tokio_postgres::types::ToSql + Sync + Send + 'static>(t: T) -> ergol::query::Filter {
                    ergol::query::Filter::Binary {
                        column: stringify!(#id_name),
                        value: Box::new(t),
                        operator: ergol::query::Operator::Eq,
                    }
                }

                /// Keeps only the results for which the column is different from the value
                /// passed as parameter.
                pub fn neq<T: ergol::tokio_postgres::types::ToSql + Sync + Send + 'static>(t: T) -> ergol::query::Filter {
                    ergol::query::Filter::Binary {
                        column: stringify!(#id_name),
                        value: Box::new(t),
                        operator: ergol::query::Operator::Neq,
                    }
                }

                /// Keeps only the results for which the column is lesser or equals the value
                /// passed as parameter.
                pub fn leq<T: ergol::tokio_postgres::types::ToSql + Sync + Send + 'static>(t: T) -> ergol::query::Filter {
                    ergol::query::Filter::Binary {
                        column: stringify!(#id_name),
                        value: Box::new(t),
                        operator: ergol::query::Operator::Leq,
                    }
                }

                /// Keeps only the results for which the column is greater or equals the value
                /// passed as parameter.
                pub fn geq<T: ergol::tokio_postgres::types::ToSql + Sync + Send + 'static>(t: T) -> ergol::query::Filter {
                    ergol::query::Filter::Binary {
                        column: stringify!(#id_name),
                        value: Box::new(t),
                        operator: ergol::query::Operator::Geq,
                    }
                }

                /// Keeps only the results for which the column is lesser than the value passed
                /// as parameter.
                pub fn lt<T: ergol::tokio_postgres::types::ToSql + Sync + Send + 'static>(t: T) -> ergol::query::Filter {
                    ergol::query::Filter::Binary {
                        column: stringify!(#id_name),
                        value: Box::new(t),
                        operator: ergol::query::Operator::Lt,
                    }
                }

                /// Keeps only the results for which the column is greater than the value passed
                /// as parameter.
                pub fn gt<T: ergol::tokio_postgres::types::ToSql + Sync + Send + 'static>(t: T) -> ergol::query::Filter {
                    ergol::query::Filter::Binary {
                        column: stringify!(#id_name),
                        value: Box::new(t),
                        operator: ergol::query::Operator::Gt,
                    }
                }

                /// Sorts the the results according to one column in ascending order.
                pub fn ascend() -> ergol::query::OrderBy {
                    ergol::query::OrderBy {
                        column: stringify!(#id_name),
                        order: ergol::query::Order::Ascend,
                    }
                }

                /// Sorts the the results according to one column in descending order.
                pub fn descend() -> ergol::query::OrderBy {
                    ergol::query::OrderBy {
                        column: stringify!(#id_name),
                        order: ergol::query::Order::Descend,
                    }
                }


            }

            #(

                /// Module that contains the helpers for the column.
                pub mod #field_names2 {

                    /// Keeps only the results for which the column equals the value passed as
                    /// parameter.
                    pub fn eq<T: ergol::tokio_postgres::types::ToSql + Sync + Send + 'static>(t: T) -> ergol::query::Filter {
                        ergol::query::Filter::Binary {
                            column: stringify!(#field_names2),
                            value: Box::new(t),
                            operator: ergol::query::Operator::Eq,
                        }
                    }

                    /// Keeps only the results for which the column is different from the value
                    /// passed as parameter.
                    pub fn neq<T: ergol::tokio_postgres::types::ToSql + Sync + Send + 'static>(t: T) -> ergol::query::Filter {
                        ergol::query::Filter::Binary {
                            column: stringify!(#field_names2),
                            value: Box::new(t),
                            operator: ergol::query::Operator::Neq,
                        }
                    }

                    /// Keeps only the results for which the column is lesser or equals the value
                    /// passed as parameter.
                    pub fn leq<T: ergol::tokio_postgres::types::ToSql + Sync + Send + 'static>(t: T) -> ergol::query::Filter {
                        ergol::query::Filter::Binary {
                            column: stringify!(#field_names2),
                            value: Box::new(t),
                            operator: ergol::query::Operator::Leq,
                        }
                    }

                    /// Keeps only the results for which the column is greater or equals the value
                    /// passed as parameter.
                    pub fn geq<T: ergol::tokio_postgres::types::ToSql + Sync + Send + 'static>(t: T) -> ergol::query::Filter {
                        ergol::query::Filter::Binary {
                            column: stringify!(#field_names2),
                            value: Box::new(t),
                            operator: ergol::query::Operator::Geq,
                        }
                    }

                    /// Keeps only the results for which the column is lesser than the value passed
                    /// as parameter.
                    pub fn lt<T: ergol::tokio_postgres::types::ToSql + Sync + Send + 'static>(t: T) -> ergol::query::Filter {
                        ergol::query::Filter::Binary {
                            column: stringify!(#field_names2),
                            value: Box::new(t),
                            operator: ergol::query::Operator::Lt,
                        }
                    }

                    /// Keeps only the results for which the column is greater than the value passed
                    /// as parameter.
                    pub fn gt<T: ergol::tokio_postgres::types::ToSql + Sync + Send + 'static>(t: T) -> ergol::query::Filter {
                        ergol::query::Filter::Binary {
                            column: stringify!(#field_names2),
                            value: Box::new(t),
                            operator: ergol::query::Operator::Gt,
                        }
                    }

                    /// Sorts the the results according to one column in ascending order.
                    pub fn ascend() -> ergol::query::OrderBy {
                        ergol::query::OrderBy {
                            column: stringify!(#field_names2),
                            order: ergol::query::Order::Ascend,
                        }
                    }

                    /// Sorts the the results according to one column in descending order.
                    pub fn descend() -> ergol::query::OrderBy {
                        ergol::query::OrderBy {
                            column: stringify!(#field_names2),
                            order: ergol::query::Order::Descend,
                        }
                    }

                    #field_likes

                }
            )*
        }
    };

    tokens
}

/// Generates some helper functions for the type.
pub fn to_impl(name: &Ident, id_field: &Field, other_fields: &[&Field]) -> TokenStream2 {
    let id_name = id_field.ident.as_ref().unwrap();

    use case::CaseExt;
    let table_name = format_ident!("{}s", name.to_string().to_snake());
    let queryable = quote! { ergol::Queryable<impl ergol::tokio_postgres::GenericClient> };
    let error = quote! { ergol::tokio_postgres::Error };

    let without_id = format_ident!("{}WithoutId", name);

    let field_comment = other_fields
        .iter()
        .map(|field| find_attribute(field, "doc"))
        .collect::<Vec<_>>();

    let names = other_fields.iter().map(|field| &field.ident);
    let names2 = names.clone();
    let names3 = names.clone();
    let names4 = names.clone();
    let names5 = names.clone();

    let names_as_strings = names
        .clone()
        .map(|x| format!("\"{}\"", x.as_ref().unwrap().to_string()))
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
        .map(|(i, name)| format!("\"{}\" = ${}", name.as_ref().unwrap(), i + 1))
        .collect::<Vec<_>>()
        .join(", ");

    let last_dollar = format!("${}", other_fields.len() + 1);

    let insert_query = format!(
        "INSERT INTO \"{}\"({}) VALUES({}) RETURNING *;",
        table_name, names_as_strings, dollars,
    );

    let update_query = format!(
        "UPDATE \"{}\" SET {} WHERE \"{}\" = {};",
        table_name,
        names_and_dollars,
        id_field.ident.as_ref().unwrap(),
        last_dollar
    );

    let delete_query = format!(
        "DELETE FROM \"{}\" WHERE \"{}\" = $1;",
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
            pub async fn save<Q: #queryable>(self, db: &Q) -> std::result::Result<#name, #error> {
                let row = db.client().query_one(#insert_query, &[ #( &self.#names4, )* ]).await?;
                Ok(<#name as ergol::ToTable>::from_row(&row))
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
            pub async fn save<Q: #queryable>(&self, db: &Q) -> std::result::Result<(), #error> {
                db.client().query(#update_query, &[ #( &self.#names5, )* &self.#id_name ]).await?;
                Ok(())
            }

            /// Deletes self from the database.
            pub async fn delete<Q: #queryable>(self, db: &Q) -> std::result::Result<(), #error> {
                db.client().query(#delete_query, &[&self.id()]).await?;
                Ok(())
            }
        }
    }
}

/// Generates the getters for the unique fields.
pub fn to_unique(name: &Ident, id_field: &Field, other_fields: &[&Field]) -> TokenStream2 {
    use case::CaseExt;
    let table_name = format_ident!("{}s", name.to_string().to_snake());

    let queryable = quote! { ergol::Queryable<impl ergol::tokio_postgres::GenericClient> };
    let error = quote! { ergol::tokio_postgres::Error };

    let fields = &[id_field];

    let fields = fields.iter().chain(other_fields.iter());

    let getters = fields
        .clone()
        .map(|field| format_ident!("get_by_{}", field.ident.as_ref().unwrap()));

    let types = fields.clone().map(|field| &field.ty);

    let queries = fields.clone().map(|field| {
        format!(
            "SELECT * FROM \"{}\" WHERE \"{}\" = $1",
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
                pub async fn #getters<T: Into<#types>, Q: #queryable>(attr: T, db: &Q) -> std::result::Result<Option<#name>, #error> {
                    let mut rows = db.client().query(#queries, &[&attr.into()]).await?;
                    Ok(rows.pop().map(|x| <#name as ToTable>::from_row(&x)))
                }
            )*
        }
    }
}

/// Struct to help parse the map_by attribute.
struct MappedBy {
    pub _paren_token: token::Paren,
    pub names: Punctuated<Ident, Token![,]>,
}

impl Parse for MappedBy {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;

        Ok(MappedBy {
            _paren_token: parenthesized!(content in input),
            names: content.parse_terminated(Ident::parse).unwrap(),
        })
    }
}

/// Changes the types of one to one fields.
pub fn fix_one_to_one_fields(name: &Ident, fields: &mut FieldsNamed) -> TokenStream2 {
    use case::CaseExt;
    let table_name = format_ident!("{}s", name.to_string().to_snake());
    let queryable = quote! { ergol::Queryable<impl ergol::tokio_postgres::GenericClient> };
    let error = quote! { ergol::tokio_postgres::Error };

    let fields_clone: FieldsNamed = fields.clone();

    let mut fields_to_fix = fields
        .named
        .iter_mut()
        .filter(|field| find_attribute(field, "one_to_one").is_some());

    let fields_clone = fields_clone
        .named
        .iter()
        .filter(|field| find_attribute(field, "one_to_one").is_some());

    let idents = fields_clone.clone().map(|x| x.ident.as_ref().unwrap());
    let types = fields_clone.clone().map(|x| &x.ty);

    let tokens = fields_clone
        .clone()
        .map(|x| find_attribute(x, "one_to_one").unwrap())
        .map(|x| Into::<TokenStream>::into(x.tokens.clone()))
        .map(|tokens| {
            let m = parse_macro_input!(tokens as MappedBy);
            let idents = m.names.into_iter().collect::<Vec<_>>();
            if idents.len() != 1 {
                panic!("one to one fields must have exactly one map by");
            }
            let name = &idents[0];
            let q = quote! { #name };
            q.into()
        })
        .map(Into::<TokenStream2>::into);

    let query = fields_clone.clone().map(|field| {
        format!(
            "SELECT * FROM \"{}\" WHERE \"{}\" = $1",
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
                pub async fn #idents<Q: #queryable>(&self, db: &Q) -> std::result::Result<#types, #error> {
                    Ok(self.#idents.fetch(db).await?)
                }
            }

            impl #types {
                #[doc=#tokens_doc]
                pub async fn #tokens<Q: #queryable>(&self, db: &Q) -> std::result::Result<Option<#name>, #error> {
                    let mut rows = db.client().query(#query, &[&self.id]).await?;
                    Ok(rows.pop().map(|x| #name::from_row(&x)))
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
    let queryable = quote! { ergol::Queryable<impl ergol::tokio_postgres::GenericClient> };
    let error = quote! { ergol::tokio_postgres::Error };

    let fields_clone: FieldsNamed = fields.clone();

    let mut fields_to_fix = fields
        .named
        .iter_mut()
        .filter(|field| find_attribute(field, "many_to_one").is_some());

    let fields_clone = fields_clone
        .named
        .iter()
        .filter(|field| find_attribute(field, "many_to_one").is_some());

    let idents = fields_clone.clone().map(|x| x.ident.as_ref().unwrap());
    let types = fields_clone.clone().map(|x| &x.ty);

    let massive_iter = fields_clone
        .clone()
        .map(|x| find_attribute(x, "many_to_one").unwrap())
        .map(|x| Into::<TokenStream>::into(x.tokens.clone()))
        .map(|tokens| {
            let m = parse_macro_input!(tokens as MappedBy);
            let idents = m.names.into_iter().collect::<Vec<_>>();
            if idents.len() > 1 {
                panic!("many to one fields must have at most one map by");
            }
            if idents.is_empty() {
                let q = quote! {};
                q.into()
            } else {
                let name = &idents[0];
                let q = quote! { #name };
                q.into()
            }
        })
        .map(Into::<TokenStream2>::into)
        .zip(types.clone())
        .zip(fields_clone.clone())
        .filter(|((x, _), _)| !x.is_empty());

    let tokens = massive_iter.clone().map(|x| x.0 .0);
    let tokens_types = massive_iter.clone().map(|x| x.0 .1);
    let tokens_fields = massive_iter.map(|x| x.1);

    let query = tokens_fields.map(|field| {
        format!(
            "SELECT * FROM \"{}\" WHERE \"{}\" = $1",
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

    let tokens_doc = tokens
        .clone()
        .into_iter()
        .zip(tokens_types.clone())
        .map(|(tokens, ty)| {
            format!(
                "Helper function to retrieve the {} from the {}.",
                quote! { #tokens }.to_string().to_snake(),
                quote! { #ty }.to_string().to_snake(),
            )
        });

    let q1 = quote! {
        #(
            impl #name {
                #[doc=#idents_doc]
                pub async fn #idents<Q: #queryable>(&self, db: &Q) -> std::result::Result<#types, #error> {
                    Ok(self.#idents.fetch(db).await?)
                }
            }
        )*
    };

    let q2 = quote! {
        #(
            impl #tokens_types {
                #[doc=#tokens_doc]
                pub async fn #tokens<Q: #queryable>(&self, db: &Q) -> std::result::Result<Vec<#name>, #error> {
                    let mut rows = db.client().query(#query, &[&self.id]).await?;
                    Ok(rows.iter().map(#name::from_row).collect::<Vec<_>>())
                }
            }
        )*
    };

    for field in &mut fields_to_fix {
        let ty = &field.ty;
        field.ty = syn::Type::Verbatim(quote! { ergol::relation::ManyToOne<#ty> });
    }

    quote! {
        #q1
        #q2
    }
}

/// Changes the types of many to many fields.
pub fn fix_many_to_many_fields(name: &Ident, fields: &FieldsNamed) -> TokenStream2 {
    use case::CaseExt;
    let table_name = format_ident!("{}s", name.to_string().to_snake());
    let queryable = quote! { ergol::Queryable<impl ergol::tokio_postgres::GenericClient> };
    let error = quote! { ergol::tokio_postgres::Error };

    let fields_to_fix = fields
        .named
        .iter()
        .filter(|field| find_attribute(field, "many_to_many").is_some());

    let extra = fields_to_fix
        .clone()
        .map(|x| find_attribute(x, "many_to_many").unwrap())
        .map(|x| Into::<TokenStream>::into(x.tokens.clone()))
        .map(|tokens| {
            let m = parse::<MappedBy>(tokens).unwrap();
            m.names.into_iter().skip(1).collect::<Vec<_>>()
        });

    let count = extra.clone().map(|x| x.len());

    let extra_rows_without_offset = count.clone().map(|count| {
        (0..count)
            .map(|i| {
                quote! { x.get(#i) }
            })
            .collect::<Vec<_>>()
    });

    let extra_snake = extra
        .clone()
        .map(|x| {
            x.into_iter()
                .map(|y| format_ident!("{}", y.to_string().to_snake()))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let extra_snake = extra_snake.iter();

    let names = fields_to_fix.clone().map(|x| &x.ident);
    let add_names = fields_to_fix.clone().map(|x| {
        format_ident!("add_{}", {
            let mut p = x.ident.as_ref().unwrap().to_string();
            p.pop();
            p
        })
    });

    let update_names = extra_snake.clone().map(|x| {
        x.into_iter()
            .map(|y| format_ident!("update_{}", y.to_string().to_snake()))
            .collect::<Vec<_>>()
    });

    let delete_names = fields_to_fix.clone().map(|x| {
        format_ident!("remove_{}", {
            let mut p = x.ident.as_ref().unwrap().to_string();
            p.pop();
            p
        })
    });

    let insert_queries = fields_to_fix
        .clone()
        .zip(extra_snake.clone())
        .map(|(x, snake)| {
            let y = format_ident!("{}_{}_join", table_name, x.ident.as_ref().unwrap()).to_string();

            let extra_columns = snake
                .iter()
                .map(|x| format!("\"{}\"", x))
                .collect::<Vec<_>>();

            let empty = extra_columns.is_empty();
            let extra_columns = extra_columns.join(",");

            let extra_dollars = snake
                .into_iter()
                .enumerate()
                .map(|(x, _)| format!("${}", x + 3))
                .collect::<Vec<_>>()
                .join(",");

            format!(
                "INSERT INTO \"{}\"(\"{}_id\", \"{}_id\" {}) VALUES ($1, $2 {});",
                y,
                table_name,
                x.ident.as_ref().unwrap(),
                if empty {
                    String::new()
                } else {
                    format!(", {}", extra_columns)
                },
                if empty {
                    String::new()
                } else {
                    format!(", {}", extra_dollars)
                },
            )
        });

    let update_queries = fields_to_fix
        .clone()
        .zip(extra_snake.clone())
        .map(|(x, snake)| {
            let y = format_ident!("{}_{}_join", table_name, x.ident.as_ref().unwrap()).to_string();

            let extra_columns = snake.iter().map(|x| x.to_string());
            let extra_dollars = snake.iter().enumerate().map(|(x, _)| format!("${}", x + 3));

            extra_columns
                .zip(extra_dollars)
                .map(|(z, t)| {
                    format!(
                        "UPDATE \"{}\" SET \"{}\" = {} WHERE \"{}_id\" = $1 AND \"{}_id\" = $2;",
                        y,
                        z,
                        t,
                        table_name,
                        x.ident.as_ref().unwrap(),
                    )
                })
                .collect::<Vec<_>>()
        });

    let delete_queries = fields_to_fix.clone().map(|x| {
        let y = format_ident!("{}_{}_join", table_name, x.ident.as_ref().unwrap()).to_string();
        format!(
            "DELETE FROM \"{}\" WHERE \"{}_id\" = $1 AND \"{}_id\" = $2 RETURNING \"id\";",
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
        .map(|x| find_attribute(x, "many_to_many").unwrap())
        .map(|x| Into::<TokenStream>::into(x.tokens.clone()))
        .map(|tokens| {
            let m = parse_macro_input!(tokens as MappedBy);
            let idents = m.names.into_iter().collect::<Vec<_>>();
            if idents.len() < 1 {
                panic!("many to many fields must have at least one attribute");
            }
            let name = &idents[0];
            let q = quote! { #name };
            q.into()
        })
        .map(Into::<TokenStream2>::into);

    let add_tokens = fields_to_fix
        .clone()
        .map(|x| find_attribute(x, "many_to_many").unwrap())
        .map(|x| Into::<TokenStream>::into(x.tokens.clone()))
        .map(|tokens| {
            let m = parse_macro_input!(tokens as MappedBy);
            let idents = m.names.into_iter().collect::<Vec<_>>();
            if idents.len() < 1 {
                panic!("many to many fields must have at least one attribute");
            }
            let name = &idents[0];
            let mut name = format!("add_{}", name.to_string());
            name.pop();
            let name = format_ident!("{}", name);
            let q = quote! { #name };
            q.into()
        })
        .map(Into::<TokenStream2>::into);

    let delete_tokens = fields_to_fix
        .clone()
        .map(|x| find_attribute(x, "many_to_many").unwrap())
        .map(|x| Into::<TokenStream>::into(x.tokens.clone()))
        .map(|tokens| {
            let m = parse_macro_input!(tokens as MappedBy);
            let idents = m.names.into_iter().collect::<Vec<_>>();
            if idents.len() < 1 {
                panic!("many to many fields must have at least one attribute");
            }
            let name = &idents[0];
            let mut name = format!("remove_{}", name.to_string());
            name.pop();
            let name = format_ident!("{}", name);
            let q = quote! { #name };
            q.into()
        })
        .map(Into::<TokenStream2>::into);

    let select_queries = fields_to_fix
        .clone()
        .zip(types_names)
        .zip(extra_snake.clone())
        .map(|((x, z), extra)| {
            let y = format_ident!("{}_{}_join", table_name, x.ident.as_ref().unwrap()).to_string();
            let extra_vars = extra
                .iter()
                .map(|x| format!("\"{}\".\"{}\"", y, x.to_string()))
                .collect::<Vec<_>>()
                .join(", ");

            format!(
                "SELECT {} {4}.* FROM \"{}\",\"{4}\" WHERE \"{}_id\" = $1 AND \"{4}\".\"id\" = \"{}_id\";",
                if extra.is_empty() {
                    String::new()
                } else {
                    format!("{}, ", extra_vars)
                },
                y,
                table_name,
                x.ident.as_ref().unwrap(),
                z,
            )
        });

    let query = fields_to_fix.zip(extra_snake.clone()).map(|(x, extra)| {
        let y = format_ident!("{}_{}_join", table_name, x.ident.as_ref().unwrap()).to_string();
        format!(
            "SELECT {} \"{}\".* FROM \"{}\",\"{}\" WHERE \"{}_id\" = $1 AND \"{}_id\" = \"{}\".\"id\";",
            if extra.is_empty() {
                String::from("")
            } else {
                format!(
                    "{}, ",
                    extra
                        .into_iter()
                        .map(|x| format!("\"{}\".\"{}\"", y, x.to_string()))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            },
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
                pub async fn #add_names<Q: #queryable>(&self, name: &#types, #(#extra_snake: #extra,)* db: &Q) -> std::result::Result<(), #error> {
                    let rows = db.client().query(#insert_queries, &[&self.id(), &name.id(), #(&#extra_snake,)*]).await?;
                    Ok(())
                }

                /// TODO fix doc
                pub async fn #delete_names<Q: #queryable>(&self, name: &#types, db: &Q) -> std::result::Result<bool, #error> {
                    let rows = db.client().query(#delete_queries, &[&self.id(), &name.id()]).await?;
                    Ok(!rows.is_empty())
                }

                /// TODO fix doc
                pub async fn #names<Q: #queryable>(&self, db: &Q) -> std::result::Result<Vec<(#types #(, #extra)*)>, #error> {
                    let rows = db.client().query(#select_queries, &[&self.id()]).await?;
                    Ok(rows.iter().map(|x| {
                        (#types::from_row_with_offset(x, #count) #(, #extra_rows_without_offset)*)
                    }).collect::<Vec<_>>())
                }

                #(
                    /// TODO fix doc
                    pub async fn #update_names<Q: #queryable>(&self, name: &#types, #extra_snake: #extra, db: &Q) -> std::result::Result<(), #error> {
                        db.client().query(#update_queries, &[&self.id(), &name.id(), &#extra_snake]).await?;
                        Ok(())
                    }
                )*
            }

            impl #types {
                /// TODO fix doc
                pub async fn #tokens<Q: #queryable>(&self, db: &Q) -> std::result::Result<Vec<(#name #(, #extra)*)>, #error> {
                    let mut rows = db.client().query(#query, &[&self.id()]).await?;
                    Ok(rows.into_iter().map(|x| {
                        (#name::from_row_with_offset(&x, #count) #(, #extra_rows_without_offset)*)
                    }).collect::<Vec<_>>())
                }

                /// TODO fix doc
                pub async fn #add_tokens<Q: #queryable>(&self, other: &#name, #(#extra_snake: #extra,)* db: &Q) -> std::result::Result<(), #error> {
                    db.client().query(#insert_queries, &[&other.id(), &self.id(), #(&#extra_snake,)*]).await?;
                    Ok(())
                }

                /// TODO fix doc
                pub async fn #delete_tokens<Q: #queryable>(&self, other: &#name, db: &Q) -> std::result::Result<bool, #error> {
                    let rows = db.client().query(#delete_queries, &[&other.id(), &self.id()]).await?;
                    Ok(!rows.is_empty())
                }

                #(
                    /// TODO fix doc
                    pub async fn #update_names<Q: #queryable>(&self, other: &#name, #extra_snake: #extra, db: &Q)  -> std::result::Result<(), #error> {
                        db.client().query(#update_queries, &[&other.id(), &self.id(), &#extra_snake]).await?;
                        Ok(())
                    }
                )*
            }
        )*
    };

    q
}
