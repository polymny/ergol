use proc_macro::TokenStream;

use syn::export::TokenStream2;
use syn::{self, Ident};

use quote::{format_ident, quote};

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

    let q = quote! {
        #impl_variants
        #impl_pg
    };

    q.into()
}

/// Adds the type_name, create_type and drop_type functions on enum type.
pub fn impl_variants(name: &Ident, variants: &[Ident]) -> TokenStream2 {
    use case::CaseExt;
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
            pub fn type_name() -> &'static str {
                stringify!(#type_name)
            }

            pub fn create_type() -> ergol::query::CreateType {
                ergol::query::CreateType(vec![String::from(#create_type)])
            }

            pub fn drop_type() -> ergol::query::DropType {
                ergol::query::DropType(vec![String::from(#drop_type)])
            }
        }
    }
}

/// Adds the implementation of the Pg, ToSql and FromSql traits for enum type.
pub fn impl_traits(name: &Ident, variants: &[Ident]) -> TokenStream2 {
    use case::CaseExt;
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
            ) -> Result<ergol::tokio_postgres::types::IsNull, Box<dyn std::error::Error + 'static + Sync + Send>> {

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
            ) -> Result<Self, Box<dyn std::error::Error + 'static + Sync + Send>> {
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
