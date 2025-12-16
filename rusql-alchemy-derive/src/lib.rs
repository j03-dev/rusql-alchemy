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

    let down = format!("drop table if exists {name};");
    let up = {
        let fields = schema_fields
            .iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        format!("create table if not exists {name} ({fields});").replace('"', "")
    };

    let delete = {
        #[cfg(not(feature = "libsql"))]
        quote!{rusql_alchemy::sqlx::query(&query).bind(self. # primary_key).execute(conn).await?;}

        #[cfg(feature = "libsql")]
        quote! {conn.execute(&query, rusql_alchemy::libsql::params![self.#primary_key]).await?;}
    };

    let expanded = quote! {
        #[rusql_alchemy::async_trait::async_trait]
        impl Model for #name {
            const UP: &'static str = #up;
            const DOWN: &'static str = #down;
            const NAME: &'static str = stringify!(#name);
            const PK: &'static str = stringify!(#primary_key);

            async fn save(&self, conn: &rusql_alchemy::db::Connection) -> Result<(), rusql_alchemy::Error> {
                Self::create(rusql_alchemy::kwargs!(#(#create_args = self.#create_args),*),conn).await
            }

            async fn update(&self, conn: &rusql_alchemy::db::Connection) -> Result<(), rusql_alchemy::Error> {
                Self::set(self.#primary_key, rusql_alchemy::kwargs!(#(#update_args = self.#update_args),*),conn).await
            }

            async fn delete(&self, conn: &rusql_alchemy::db::Connection) -> Result<(), rusql_alchemy::Error> {
                let query = format!("delete from {} where {}=?1;", Self::NAME, Self::PK).replace("?", rusql_alchemy::db::PLACEHOLDER);
                #delete
                Ok(())
            }
        }

        impl Default for #name {
            fn default() -> Self {
                Self {#(#default_fields),*}
            }
        }

        rusql_alchemy::inventory::submit! {
            rusql_alchemy::MigrationRegistrar {
                migrate_fn: #name::migrate
            }
        }
    };

    expanded.into()
}
