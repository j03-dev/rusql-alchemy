use codegen::{process_fields, ModelData};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

mod codegen;

#[proc_macro_derive(Model, attributes(field))]
pub fn model_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let fields = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => &fields.named,
            _ => panic!("Model derive macro only supports structs with named fields"),
        },
        _ => panic!("Model derive macro only supports structs"),
    };

    let ModelData {
        schema_fields,
        create_args,
        update_args,
        the_primary_key,
        default_fields,
    } = process_fields(fields);

    let primary_key = {
        let pk = the_primary_key.to_string();
        quote! {
            const PK: &'static str = #pk;
        }
    };

    let schema = {
        let fields = schema_fields
            .iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let schema = format!("create table if not exists {name} ({fields});").replace('"', "");

        quote! {
            const SCHEMA: &'static str = #schema;
        }
    };

    let save = quote! {
        async fn save(&self, conn: &Connection) -> Result<(), sqlx::Error> {
            Self::create(
                kwargs!(
                    #(#create_args = self.#create_args),*
                ),
                conn,
            )
            .await
        }
    };

    let update = quote! {
        async fn update(&self, conn: &Connection) -> Result<(), sqlx::Error> {
            Self::set(
                self.#the_primary_key.clone(),
                kwargs!(
                    #(#update_args = self.#update_args),*
                ),
                conn,
            )
            .await
        }
    };

    let delete = {
        let query = format!("delete from {name} where {the_primary_key}=?1;");
        quote! {
            async fn delete(&self, conn: &Connection) -> Result<(), sqlx::Error> {
                let placeholder = rusql_alchemy::PLACEHOLDER.to_string();
                sqlx::query(&#query.replace("?", &placeholder))
                    .bind(self.#the_primary_key.clone())
                    .execute(conn)
                    .await?;
                Ok(())
            }
        }
    };

    let expanded = quote! {
        #[async_trait]
        impl Model for #name {
            const NAME: &'static str = stringify!(#name);
            #schema
            #primary_key
            #save
            #update
            #delete
        }

        impl Default for #name {
            fn default() -> Self {
                Self {
                    #(#default_fields),*
                }
            }
        }

        rusql_alchemy::prelude::inventory::submit! {
            MigrationRegistrar {
                migrate_fn: #name::migrate
            }
        }
    };

    expanded.into()
}
