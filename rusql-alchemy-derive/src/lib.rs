use deluxe::ExtractAttributes;
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, Data, DeriveInput, Field, Fields, GenericArgument, PathArguments, Type,
};

#[derive(ExtractAttributes, Default, Debug)]
#[deluxe(attributes(field))]
struct ModelField {
    primary_key: Option<bool>,
    auto: Option<bool>,
    size: Option<usize>,
    unique: Option<bool>,
    default: Option<TokenStream>,
    foreign_key: Option<TokenStream>,
}

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

    let create = {
        quote! {
            async fn save(&self, conn: &Connection) -> Result<(), sqlx::Error> {
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
            async fn update(&self, conn: &Connection) -> Result<(), sqlx::Error> {
                Self::set(
                    self.#the_primary_key,
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
        let query = format!("delete from {name} where {the_primary_key}=?1;");
        quote! {
            async fn delete(&self, conn: &Connection) -> Result<(), sqlx::Error> {
                let placeholder = rusql_alchemy::PLACEHOLDER.to_string();
                sqlx::query(&#query.replace("?", &placeholder))
                    .bind(self.#the_primary_key)
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
            #create
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

#[derive(Debug)]
struct ModelData {
    schema_fields: Vec<proc_macro2::TokenStream>,
    create_args: Vec<proc_macro2::TokenStream>,
    update_args: Vec<proc_macro2::TokenStream>,
    the_primary_key: proc_macro2::TokenStream,
    default_fields: Vec<proc_macro2::TokenStream>,
}

fn process_fields(fields: &syn::punctuated::Punctuated<Field, syn::Token![,]>) -> ModelData {
    let mut schema_fields = Vec::new();
    let mut create_args = Vec::new();
    let mut update_args = Vec::new();
    let mut the_primary_key = quote! {};
    let mut default_fields = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().expect("Field name should be present");
        let field_type = extract_inner_type(&field.ty);

        let mut obj = field.clone();
        let attrs = ModelField::extract_attributes(&mut obj).unwrap_or_default();

        let is_auto = attrs.auto.unwrap_or(false);
        let is_unique = attrs.unique.unwrap_or(false);
        let size = attrs.size;
        let is_primary_key = attrs.primary_key.unwrap_or(false);

        let is_nullable = match &field.ty {
            syn::Type::Path(type_path) => {
                if let Some(segment) = type_path.path.segments.last() {
                    segment.ident == "Option"
                } else {
                    false
                }
            }
            _ => false,
        };

        let foreign_key = match &attrs.foreign_key {
            Some(fk) => {
                let split_fk = fk.to_string();
                let foreign_key_parts: Vec<&str> = split_fk.split('.').collect();
                if foreign_key_parts.len() != 2 {
                    panic!("Invalid foreign key");
                }
                let foreign_key_table = foreign_key_parts[0];
                let foreign_key_field = foreign_key_parts[1];

                quote! {
                    references #foreign_key_table(#foreign_key_field)
                }
            }
            None => quote! {},
        };

        if is_primary_key {
            the_primary_key = quote! { #field_name };
        }

        let default = match &attrs.default {
            Some(default_value) => {
                let default_value = default_value.to_string().replace('"', "");
                match default_value.as_str() {
                    "now" => {
                        if field_type == "Date" {
                            quote! { default current_date }
                        } else if field_type == "DateTime" {
                            quote! { default current_timestamp }
                        } else {
                            panic!("'now' is work only with Date or DateTime");
                        }
                    }
                    "false" | "true" if field_type == "Boolean" => {
                        if default_value == "true" {
                            quote! {default 1}
                        } else {
                            quote! {default 0}
                        }
                    }
                    _ => {
                        quote! { default #default_value }
                    }
                }
            }
            None => quote! {},
        };

        let default_value = match &attrs.default {
            Some(default_value) => {
                let default_value_str = default_value.to_string().replace('"', "");
                match default_value_str.as_str() {
                    "now" => {
                        if field_type == "Date" {
                            quote! { chrono::Utc::now().format("%Y-%m-%d").to_string() }
                        } else if field_type == "DateTime" {
                            quote! { chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string() }
                        } else {
                            panic!("'now' is work only with Date or DateTime");
                        }
                    }
                    "false" | "true" if field_type == "Boolean" => {
                        if default_value_str == "true" {
                            quote! {1}
                        } else {
                            quote! {0}
                        }
                    }
                    _ => {
                        if is_nullable {
                            quote! {Some(#default_value_str.into())}
                        } else {
                            quote! {#default_value_str.into()}
                        }
                    }
                }
            }
            None => {
                if is_nullable {
                    quote! {None}
                } else {
                    match field_type.as_str() {
                        "Serial" | "Integer" => quote! {0},
                        "String" | "Text" => quote! {String::default()},
                        "Float" => quote! {0.0},
                        "Date" | "Datetime" => quote! {String::default()},
                        "Boolean" => quote! {0},
                        _ => panic!("Unsupported type for default value"),
                    }
                }
            }
        };

        let field_schema = generate_field_schema(
            field_name,
            &field_type,
            attrs.primary_key.unwrap_or(false),
            is_auto,
            is_unique,
            is_nullable,
            size,
            &default,
            &foreign_key,
            &mut create_args,
            &mut update_args,
        );

        schema_fields.push(field_schema);
        default_fields.push(quote! {#field_name: #default_value});
    }

    ModelData {
        schema_fields,
        create_args,
        update_args,
        the_primary_key,
        default_fields,
    }
}

#[allow(clippy::too_many_arguments)]
fn generate_field_schema(
    field_name: &syn::Ident,
    field_type: &str,
    is_primary_key: bool,
    is_auto: bool,
    is_unique: bool,
    is_nullable: bool,
    size: Option<usize>,
    default: &proc_macro2::TokenStream,
    foreign_key: &proc_macro2::TokenStream,
    create_args: &mut Vec<proc_macro2::TokenStream>,
    update_args: &mut Vec<proc_macro2::TokenStream>,
) -> proc_macro2::TokenStream {
    let base_type = match field_type {
        "Serial" => quote! { serial },
        "Integer" => quote! { integer },
        "String" => {
            if let Some(size) = size {
                let size = size.to_string();
                quote! { varchar(#size) }
            } else {
                quote! { varchar(255) }
            }
        }
        "Float" => quote! { float },
        "Text" => quote! { text },
        "Date" => quote! { varchar(10) },
        "Boolean" => quote! { integer },
        "DateTime" => quote! { varchar(40) },
        p_type => panic!(
            r#"Unexpected field type: '{}'.
            Expected one of: 'Serial', 'Integer', 'String', 'Float', 'Text', 'Date', 'Boolean', 'DateTime'.
            Please check the field type."#,
            p_type
        ),
    };

    let primary_key = if is_primary_key {
        let auto = if is_auto {
            quote! { autoincrement }
        } else if field_type == "Serial" {
            quote! {}
        } else {
            create_args.push(quote! { #field_name });
            quote! {}
        };
        quote! { primary key #auto }
    } else {
        create_args.push(quote! { #field_name });
        update_args.push(quote! { #field_name });
        quote! {}
    };

    let nullable = if is_nullable {
        quote! {}
    } else {
        quote! { not null }
    };
    let unique = if is_unique {
        quote! { unique }
    } else {
        quote! {}
    };

    quote! { #field_name #base_type #primary_key #unique #default #nullable #foreign_key }
}

fn extract_inner_type(field_type: &Type) -> String {
    match field_type {
        Type::Path(type_path) => {
            let last_segment = type_path
                .path
                .segments
                .last()
                .expect("Type path should have at least one segment");
            if last_segment.ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &last_segment.arguments {
                    if let Some(GenericArgument::Type(inner_type)) = args.args.first() {
                        return extract_inner_type(inner_type);
                    }
                }
            }
            last_segment.ident.to_string()
        }
        _ => panic!("Unsupported field type"),
    }
}
