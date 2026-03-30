use deluxe::ExtractAttributes;
use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

mod process;

#[derive(ExtractAttributes, Default, Debug)]
#[deluxe(attributes(trigger))]
struct TriggerField {
    name: String,
    event: String,
    on: String,
    action: String,
}

#[proc_macro_derive(Model, attributes(field, trigger))]
pub fn model_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let trigger_attr = TriggerField::extract_attributes(&mut input.clone()).ok();

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

    let mut up = {
        let fields = schema_fields
            .iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        format!("create table if not exists {name} ({fields});").replace('"', "")
    };
    let mut down = format!("drop table if exists {name};");

    if let Some(attr) = trigger_attr {
        up.push_str(&format!(
            "create trigger if not exists {name} {event} on {on} BEGIN {action} END;",
            name = attr.name,
            event = attr.event,
            on = attr.on,
            action = attr.action,
        ));

        down.push_str(&format!("drop trigger if exists {name};", name = attr.name));
   }

    let delete = {
        #[cfg(not(feature = "libsql"))]
        quote! {::rusql_alchemy::sqlx::query(&query).bind(self.#primary_key.clone()).execute(conn).await?;}

        #[cfg(feature = "libsql")]
        quote! {conn.execute(&query, ::rusql_alchemy::libsql::params![self.#primary_key.clone()]).await?;}
    };

    let expanded = quote! {
        #[rusql_alchemy::async_trait::async_trait]
        impl Model for #name {
            const UP: &'static str = #up;
            const DOWN: &'static str = #down;
            const NAME: &'static str = stringify!(#name);
            const PK: &'static str = stringify!(#primary_key);

            async fn save(&self, conn: &::rusql_alchemy::db::Connection) -> Result<(), ::rusql_alchemy::Error> {
                Self::create(::rusql_alchemy::kwargs!(#(#create_args = self.#create_args),*),conn).await
            }

            async fn update(&self, conn: &::rusql_alchemy::db::Connection) -> Result<(), ::rusql_alchemy::Error> {
                Self::set(self.#primary_key.clone(), ::rusql_alchemy::kwargs!(#(#update_args = self.#update_args),*),conn).await
            }

            async fn delete(&self, conn: &::rusql_alchemy::db::Connection) -> Result<(), ::rusql_alchemy::Error> {
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
            ::rusql_alchemy::MigrationRegistrar {
                up_fn: #name::up,
                down_fn: #name::down,
            }
        }
    };

    expanded.into()
}
