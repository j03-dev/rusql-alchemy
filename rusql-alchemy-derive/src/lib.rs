use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

mod process;

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

    let process::Output {
        primary_key,
        default_fields,
        schema_fields,
        create_args,
        update_args,
    } = process::process_fields(fields);

    let pk = primary_key.to_string();

    let schema = {
        let fields = schema_fields
            .iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        format!("create table if not exists {name} ({fields});").replace('"', "")
    };

    let expanded = quote! {
        #[async_trait]
        impl Model for #name {
            const NAME: &'static str = stringify!(#name);
            const PK: &'static str = #pk;
            const SCHEMA: &'static str = #schema;

            async fn save(&self, conn: &Connection) -> Result<(), rusql_alchemy::Error> {
                Self::create(kwargs!(#(#create_args = self.#create_args),*),conn).await
            }

            async fn update(&self, conn: &Connection) -> Result<(), rusql_alchemy::Error> {
                Self::set(self.#primary_key,kwargs!(#(#update_args = self.#update_args),*),conn).await
            }

            async fn delete(&self, conn: &Connection) -> Result<(), rusql_alchemy::Error> {
                let query = format!("delete from {} where {}=?1;", Self::NAME, Self::PK).replace("?", rusql_alchemy::PLACEHOLDER);

                #[cfg(not(feature = "turso"))]
                {
                    sqlx::query(&query)
                        .bind(self.#primary_key)
                        .execute(conn)
                        .await?;
                }

                #[cfg(feature = "turso")]
                conn.execute(&query, rusql_alchemy::params![self.#primary_key]).await?;

                Ok(())
            }
        }

        impl Default for #name {
            fn default() -> Self {
                Self {#(#default_fields),*}
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
