//! Derive macro for the `Model` trait.
//!
//! `#[derive(Model)]` (with `#[field(...)]` attributes) implements
//! `rusql_alchemy::db::model::Model` for a struct and:
//!
//! * builds `SCHEMA` (`create table if not exists ...`) from the fields,
//! * sets `NAME` (the struct name) and `PK` (the `primary_key` field),
//! * implements `save`, `update` and `delete`,
//! * implements `Default` for the struct,
//! * submits the schema to `inventory`, so `Database::up()` and
//!   `rusql-alchemy-migration` discover the model.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

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

    let schema = {
        let fields = schema_fields
            .iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        format!("create table if not exists {name} ({fields});").replace('"', "")
    };

    let delete = {
        #[cfg(not(feature = "libsql"))]
        quote! {::rusql_alchemy::sqlx::query(::rusql_alchemy::sqlx::AssertSqlSafe(query)).bind(self.#primary_key.clone()).execute(conn).await?;}

        #[cfg(feature = "libsql")]
        quote! {conn.execute(&query, ::rusql_alchemy::libsql::params![self.#primary_key.clone()]).await?;}
    };

    let save = {
        #[cfg(not(feature = "libsql"))]
        quote! {
            async fn save<'e, E>(&self, conn: E) -> ::std::result::Result<Self, ::rusql_alchemy::Error>
            where
                Self: Sized,
                E: sqlx::Executor<'e, Database = sqlx::Any> + Send,
            {
                Self::create(
                    ::rusql_alchemy::kwargs!(#(#create_args = self.#create_args),*),
                    conn,
                )
                .await
            }
        }

        #[cfg(feature = "libsql")]
        quote! {
            async fn save(
                &self,
                conn: &::rusql_alchemy::db::Connection,
            ) -> ::std::result::Result<Self, ::rusql_alchemy::Error>
            where
                Self: Sized,
            {
                Self::create(
                    ::rusql_alchemy::kwargs!(#(#create_args = self.#create_args),*),
                    conn,
                )
                .await
            }
        }
    };

    let expanded = quote! {
        #[rusql_alchemy::async_trait::async_trait]
        impl Model for #name {
            const SCHEMA: &'static str = #schema;
            const NAME: &'static str = stringify!(#name);
            const PK: &'static str = stringify!(#primary_key);

            #save

            async fn update(&self, conn: &::rusql_alchemy::db::Connection) -> ::std::result::Result<(), ::rusql_alchemy::Error> {
                Self::set(self.#primary_key.clone(), ::rusql_alchemy::kwargs!(#(#update_args = self.#update_args),*),conn).await
            }

            async fn delete(&self, conn: &::rusql_alchemy::db::Connection) -> ::std::result::Result<(), ::rusql_alchemy::Error> {
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

        ::rusql_alchemy::inventory::submit! {
            ::rusql_alchemy::Schemas {
                schema: #name::SCHEMA,
            }
        }
    };

    expanded.into()
}
