use proc_macro::TokenStream;

use syn::{parse_macro_input, DeriveInput};

mod ergol;
mod pgenum;

#[proc_macro_attribute]
pub fn ergol(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    ergol::generate(input)
}

#[proc_macro_derive(PgEnum)]
pub fn derive_pgenum(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    pgenum::generate(&ast)
}
