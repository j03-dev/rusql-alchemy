extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(Model, attributes(model))]
pub fn derive_model(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;
    let fields = match &ast.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("Expected a struct with named fields"),
        },
        _ => panic!("Expected a struct"),
    };

    let field_names = fields.iter().map(|field| &field.ident).collect::<Vec<_>>();

    let gen = quote! {
        #[async_trait::async_trait]
        impl Model for #name {
            fn name() -> String {
                stringify!(#name).to_string()
            }

            async fn save(&self) -> bool {
                let fields = vec![#(stringify!(#field_names)),*];
                let values = vec![#(self.#field_names.to_string()),*];
                let query = format!(
                    "insert into {} ({}) values ({})",
                    Self::name(),
                    fields.join(", "),
                    (0..fields.len()).map(|_| "?").collect::<Vec<_>>().join(", ")
                );
                let conn = Self::conn().as_mut().await;
                conn.execute(&query, values).await.is_ok()
            }
        }
    };

    gen.into()
}
