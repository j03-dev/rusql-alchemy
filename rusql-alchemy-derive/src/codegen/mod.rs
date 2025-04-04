mod schema;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{punctuated::Punctuated, Field, GenericArgument, PathArguments, Token, Type};

use deluxe::ExtractAttributes;

use crate::codegen::schema::generate_field_schema;

pub struct ModelData {
    pub the_primary_key: TokenStream,
    pub create_args: Vec<TokenStream>,
    pub update_args: Vec<TokenStream>,
    pub schema_fields: Vec<TokenStream>,
    pub default_fields: Vec<TokenStream>,
}

fn determin_if_nullable(ty: &Type) -> bool {
    matches!(ty, Type::Path(type_path) if type_path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "Option"))
}

fn generate_foreign_key(option_foreign_key: &Option<TokenStream>) -> TokenStream {
    match option_foreign_key {
        Some(fk) => {
            let fkstr = fk.to_string();
            let foreign_key_parts: Vec<_> = fkstr.split(".").collect();
            (foreign_key_parts.len() != 2).then(|| panic!("Invalid foreign key"));
            let foreign_key_table = foreign_key_parts[0];
            let foreign_key_field = foreign_key_parts[1];
            quote! { references #foreign_key_table(#foreign_key_field) }
        }
        None => quote! {},
    }
}

fn generate_default_value(
    option_default: &Option<TokenStream>,
    is_nullable: bool,
    field_type: &str,
) -> TokenStream {
    match option_default {
        Some(default_value) => {
            let default_value = default_value.to_string().replace('"', "");
            if default_value == "now" {
                match field_type {
                    "Date" => quote! { chrono::Utc::now().format("%Y-%m-%d").to_string() },
                    "DateTime" => {
                        quote! { chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string() }
                    }
                    _ => panic!("'now' is work with Date or DateTime only"),
                }
            } else if field_type == "Boolean" {
                match default_value.as_str() {
                    "true" => quote! { 1 },
                    "false" => quote! { 0 },
                    _ => panic!("Invalid boolean default value, use 'true' or 'false'"),
                }
            } else if is_nullable {
                quote! { Some(#default_value.into()) }
            } else {
                quote! { #default_value.into() }
            }
        }
        None => is_nullable
            .then(|| quote! { None })
            .unwrap_or_else(|| match field_type {
                "Float" => quote! { 0.0 },
                "Boolean" => quote! { 0 },
                "Serial" | "Integer" => quote! { 0 },
                "String" | "Text" => quote! { String::default() },
                "Date" | "DateTime" => quote! { String::default() },
                _ => panic!("Unsupported type for default value"),
            }),
    }
}

#[derive(ExtractAttributes, Default, Debug)]
#[deluxe(attributes(field))]
pub struct ModelField {
    primary_key: Option<bool>,
    auto: Option<bool>,
    size: Option<usize>,
    unique: Option<bool>,
    default: Option<TokenStream>,
    foreign_key: Option<TokenStream>,
}

pub fn process_fields(fields: &Punctuated<Field, Token![,]>) -> ModelData {
    let mut the_primary_key = TokenStream::new();
    let mut default_fields = Vec::new();
    let mut create_args = Vec::new();
    let mut update_args = Vec::new();
    let mut schema_fields = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().expect("Field name should be present");
        let field_type = extract_inner_type(&field.ty);

        let mut obj = field.clone();
        let attrs = ModelField::extract_attributes(&mut obj).unwrap_or_default();

        let option_size = attrs.size;
        let option_default = attrs.default;
        let is_auto = attrs.auto.unwrap_or(false);
        let is_unique = attrs.unique.unwrap_or(false);
        let is_pk = attrs.primary_key.unwrap_or(false);
        let is_nullable = determin_if_nullable(&field.ty);

        let foreign_key = generate_foreign_key(&attrs.foreign_key);

        is_pk.then(|| the_primary_key = quote! { #field_name });

        let default_value = generate_default_value(&option_default, is_nullable, &field_type);
        default_fields.push(quote! { #field_name: #default_value });

        let field_schema = generate_field_schema(
            field_name,
            &field_type,
            is_pk,
            is_auto,
            is_nullable,
            is_unique,
            foreign_key,
            option_size,
            option_default,
            &mut create_args,
            &mut update_args,
        );

        schema_fields.push(field_schema);
    }

    ModelData {
        the_primary_key,
        create_args,
        update_args,
        schema_fields,
        default_fields,
    }
}

fn extract_inner_type(field_type: &Type) -> String {
    if let Type::Path(type_path) = field_type {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(GenericArgument::Type(inner_type)) = args.args.first() {
                        return extract_inner_type(inner_type);
                    }
                }
            }
            return segment.ident.to_string();
        }
    }
    panic!("Unsupported field type");
}
