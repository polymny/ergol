use std::fs::{create_dir_all, File};
use std::io::Write;

use case::CaseExt;

use proc_macro::TokenStream;

use proc_macro2::TokenStream as TokenStream2;

use syn::{self, Ident};

use quote::{format_ident, quote};

use ergol_core::{Element, Enum};

/// Generates functions and trait implementations for enum types.
pub fn generate(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let variants = match &ast.data {
        syn::Data::Enum(syn::DataEnum { variants, .. }) => variants,
        _ => panic!("Expected enum"),
    };

    let variants = variants.iter().map(|x| x.ident.clone()).collect::<Vec<_>>();

    let impl_variants = impl_variants(&name, variants.as_slice());
    let impl_pg = impl_traits(&name, variants.as_slice());

    let json = Element::Enum(Enum {
        name: format!("{}", name).to_snake(),
        variants: variants
            .into_iter()
            .map(|x| format!("{}", x).to_snake())
            .collect(),
    });

    create_dir_all("migrations/current").unwrap();
    let mut file = File::create(format!("migrations/current/{}.json", &name)).unwrap();
    file.write_all(
        serde_json::to_string_pretty(&vec![json])
            .unwrap()
            .as_bytes(),
    )
    .unwrap();

    let q = quote! {
        #impl_variants
        #impl_pg
    };

    q.into()
}

/// Adds the type_name, create_type and drop_type functions on enum type.
pub fn impl_variants(name: &Ident, variants: &[Ident]) -> TokenStream2 {
    let type_name = format_ident!("{}", name.to_string().to_snake());
    let variants_names = variants
        .iter()
        .map(|x| x.to_string().to_snake())
        .collect::<Vec<_>>();

    let create_type = format!(
        "CREATE TYPE {} AS ENUM ('{}');",
        type_name,
        variants_names.join("', '")
    );

    let drop_type = format!("DROP TYPE {} CASCADE;", type_name);

    quote! {
        impl #name {
            /// Returns the name of the type in the database.
            pub fn type_name() -> &'static str {
                stringify!(#type_name)
            }

            /// Returns the SQL query that creates the type.
            pub fn create_type() -> ergol::query::CreateType {
                ergol::query::CreateType(vec![String::from(#create_type)])
            }

            /// Returns the SQL query that drops the type.
            pub fn drop_type() -> ergol::query::DropType {
                ergol::query::DropType(vec![String::from(#drop_type)])
            }
        }
    }
}

/// Adds the implementation of the Pg, ToSql and FromSql traits for enum type.
pub fn impl_traits(name: &Ident, variants: &[Ident]) -> TokenStream2 {
    let type_name = format_ident!("{}", name.to_string().to_snake());

    let snake_variants = variants.iter().map(|x| x.to_string().to_snake());
    let snake_variants2 = snake_variants.clone();

    let impl_pg = quote! {
        impl ergol::pg::Pg for #name {
            fn ty() -> String {
                String::from(stringify!(#type_name))
            }
        }
    };

    let impl_to_sql = quote! {
        impl ergol::tokio_postgres::types::ToSql for #name {
            fn to_sql(
                &self,
                ty: &ergol::tokio_postgres::types::Type,
                out: &mut ergol::bytes::BytesMut
            ) -> std::result::Result<ergol::tokio_postgres::types::IsNull, Box<dyn std::error::Error + 'static + Sync + Send>> {

                use ergol::bytes::BufMut;

                let s = match self {
                    #(
                        #name::#variants => #snake_variants,
                    )*
                };
                out.put_slice(s.as_bytes());
                Ok(ergol::tokio_postgres::types::IsNull::No)
            }

            fn accepts(ty: &ergol::tokio_postgres::types::Type) -> bool {
                true
            }

            ergol::tokio_postgres::types::to_sql_checked!();
        }
    };

    let impl_from_sql = quote! {

        impl<'a> ergol::tokio_postgres::types::FromSql<'a> for #name {
            fn from_sql(
                ty: &ergol::tokio_postgres::types::Type,
                raw: &'a [u8]
            ) -> std::result::Result<Self, Box<dyn std::error::Error + 'static + Sync + Send>> {
                let s = std::str::from_utf8(raw).unwrap();
                match s.as_ref() {
                    #(
                        #snake_variants2 => Ok(#name::#variants),
                    )*
                    _ => unreachable!(),
                }
            }

            fn accepts(ty: &ergol::tokio_postgres::types::Type) -> bool {
                true
            }
        }


    };

    quote! {
        #impl_pg
        #impl_to_sql
        #impl_from_sql
    }
}
