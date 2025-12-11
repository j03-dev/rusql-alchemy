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

    let pk = {
        let pk = primary_key.to_string();
        quote! { const PK: &'static str = #pk; }
    };

    let schema = {
        let fields = schema_fields
            .iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let schema = format!("create table if not exists {name} ({fields});").replace('"', "");

        quote! { const SCHEMA: &'static str = #schema; }
    };

    let save = {
        quote! {
            async fn save(&self, conn: &Connection) -> Result<(), rusql_alchemy::Error> {
                Self::create(
                    kwargs!(
                        #(#create_args = self.#create_args),*
                    ),
                    conn,
                )
                .await
            }
        }
    };

    let update = {
        quote! {
            async fn update(&self, conn: &Connection) -> Result<(), rusql_alchemy::Error> {
                Self::set(
                    self.#primary_key,
                    kwargs!(
                        #(#update_args = self.#update_args),*
                    ),
                    conn,
                )
                .await
            }
        }
    };

    let delete = {
        let query = format!("delete from {name} where {primary_key}=?1;");
        #[cfg(not(feature = "turso"))]
        quote! {
            async fn delete(&self, conn: &Connection) -> Result<(), rusql_alchemy::Error> {
                sqlx::query(&#query.replace("?", rusql_alchemy::PLACEHOLDER))
                    .bind(self.#primary_key.clone())
                    .execute(conn)
                    .await?;
                Ok(())
            }
        }

        #[cfg(feature = "turso")]
        quote! {
            async fn delete(&self, conn: &Connection) -> Result<(), rusql_alchemy::Error> {
                conn.execute(&#query.replace("?", rusql_alchemy::PLACEHOLDER), rusql_alchemy::params![self.#primary_key.clone()]).await?;
                Ok(())
            }
        }
    };

    let expanded = quote! {
        #[async_trait]
        impl Model for #name {
            const NAME: &'static str = stringify!(#name);
            #schema
            #pk
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
