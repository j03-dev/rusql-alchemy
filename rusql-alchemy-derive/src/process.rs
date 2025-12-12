use deluxe::ExtractAttributes;
use proc_macro2::TokenStream;
use quote::quote;

pub struct Output {
    pub primary_key: TokenStream,
    pub default_fields: Vec<TokenStream>,
    pub schema_fields: Vec<TokenStream>,
    pub create_args: Vec<TokenStream>,
    pub update_args: Vec<TokenStream>,
}

pub fn process_fields(fields: &syn::punctuated::Punctuated<syn::Field, syn::Token![,]>) -> Output {
    let mut primary_key = TokenStream::new();
    let mut default_fields = Vec::new();
    let mut schema_fields = Vec::new();

    let mut create_args = Vec::new();
    let mut update_args = Vec::new();

    for field in fields {
        let attributes = ModelField::extract_attributes(&mut field.clone()).unwrap_or_default();
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;

        if attributes.primary_key.unwrap_or(false) {
            primary_key = quote! { #field_name };
            // if not autoincrement push to create candidate
            if !attributes.auto.unwrap_or(false) || extract_inner_type(field_type) != "Serial" {
                create_args.push(quote! { #field_name });
            }
        } else {
            create_args.push(quote! { #field_name });
            update_args.push(quote! { #field_name });
        }

        let field_schema = generate_field_schema(&attributes, field_name, field_type);
        schema_fields.push(field_schema);

        let default_field = generate_default_field(&attributes.default, field_name, field_type);
        default_fields.push(default_field);
    }

    Output {
        primary_key,
        default_fields,
        schema_fields,
        create_args,
        update_args,
    }
}

#[derive(ExtractAttributes, Default, Debug)]
#[deluxe(attributes(field))]
struct ModelField {
    primary_key: Option<bool>,
    auto: Option<bool>,
    unique: Option<bool>,
    size: Option<usize>,
    default: Option<TokenStream>,
    foreign_key: Option<TokenStream>,
}

fn generate_field_schema(
    attributes: &ModelField,
    field_name: &syn::Ident,
    field_type: &syn::Type,
) -> TokenStream {
    let inner_type = extract_inner_type(field_type);

    let sql_type = construct_sql_type(&inner_type, attributes.size);
    let primary_key = construct_primary_key(&inner_type, &attributes.primary_key, &attributes.auto);
    let unique = construct_unique(&attributes.unique);
    let default = construct_default_sql_value(&attributes.default, &inner_type);
    let nullable = construct_nullable(field_type);
    let foreign_key = construct_foreign_key(&attributes.foreign_key);

    quote! { #field_name #sql_type #primary_key #unique #default #nullable #foreign_key }
}

fn construct_primary_key(
    inner_type: &str,
    is_primary_key: &Option<bool>,
    is_auto: &Option<bool>,
) -> TokenStream {
    if is_primary_key.unwrap_or(false) {
        let auto = match (is_auto, inner_type) {
            (Some(true), _) => quote! { autoincrement },
            (None, "Serial") | _ => quote! {},
        };
        quote! { primary key #auto }
    } else {
        quote! {}
    }
}

fn construct_foreign_key(foreign_key: &Option<TokenStream>) -> TokenStream {
    match foreign_key {
        Some(fk) => match fk.to_string().split_once(".") {
            Some((table, field)) => quote! { references #table(#field) },
            _ => panic!("Invalid foreign key format"),
        },
        _ => quote! {},
    }
}

fn construct_sql_type(inner_type: &str, size: Option<usize>) -> TokenStream {
    match inner_type {
        "Text" => quote! { text },
        "Float" => quote! { float },
        "Boolean" => quote! { bool },
        "Serial" => quote! { serial },
        "Integer" => quote! { integer },
        "Date" => quote! { varchar(10) },
        "DateTime" => quote! { varchar(40) },
        "String" => match size {
            Some(s) => {
                let s = s.to_string();
                quote! { varchar(#s) }
            }
            None => quote! { varchar(255)},
        },
        other => panic!(
            "Unsupported type: {}, only 'Text' 'String' 'Float' 'Boolean' 'Serial' 'Integer' 'Date' 'DateTime' are available!",
            other
        ),
    }
}

fn construct_nullable(ty: &syn::Type) -> TokenStream {
    if !is_nullable(ty) {
        quote! { not null }
    } else {
        quote! {}
    }
}

fn is_nullable(ty: &syn::Type) -> bool {
    matches!(ty, syn::Type::Path(type_path) if type_path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "Option"))
}

fn construct_unique(unique: &Option<bool>) -> TokenStream {
    if unique.unwrap_or(false) {
        quote! { unique }
    } else {
        quote! {}
    }
}

// Default for SQL
fn construct_default_sql_value(default: &Option<TokenStream>, inner_type: &str) -> TokenStream {
    match default {
        Some(value) => {
            let value = value.to_string().replace('"', "");
            match (inner_type, value.as_str()) {
                ("Date", "now") => quote! { default current_date},
                ("DateTime", "now") => quote! { default current_timestamp },
                ("Boolean", "true") => quote! { default 1 },
                ("Boolean", "false") => quote! { default 0 },
                (_, "now") => panic!("The keyword 'now' only works with Date or DateTime type!"),
                ("Boolean", _) => panic!("Invalid boolean default value, use 'true' or 'false'!"),
                _ => quote! { default #value },
            }
        }
        None => quote! {},
    }
}

// Default for Rust `Default` impl
fn generate_default_field(
    default: &Option<TokenStream>,
    field_name: &syn::Ident,
    field_type: &syn::Type,
) -> TokenStream {
    let inner_type = extract_inner_type(field_type);
    let nullable = is_nullable(field_type);

    let default_value = match default {
        Some(value) => {
            let value = value.to_string().replace('"', "");
            match (inner_type.as_str(), value.as_str()) {
                ("Date", "now") => quote! { chrono::Utc::now().format("%Y-%m-%d").to_string() },
                ("DateTime", "now") => {
                    quote! { chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string() }
                }
                ("Boolean", "true") => quote! { 1 },
                ("Boolean", "false") => quote! { 0 },
                (_, "now") => panic!("The keyword 'now' only works with Date or DateTime type!"),
                ("Boolean", _) => panic!("Invalid boolean default value, use 'true' or 'false'!"),
                _ if nullable => quote! { Some(#value.into()) },
                _ => quote! { #value.into() },
            }
        }
        None => {
            if !nullable {
                match inner_type.as_str() {
                    "Float" => quote! { 0.0 },
                    "Boolean" => quote! { 0 },
                    "Serial" | "Integer" => quote! { 0 },
                    "String" | "Text" => quote! { String::default() },
                    "Date" | "DateTime" => quote! { String::default() },
                    _ => panic!("Unsupported type for default value"),
                }
            } else {
                quote! { None }
            }
        }
    };

    quote! { #field_name: #default_value }
}

fn extract_inner_type(field_type: &syn::Type) -> String {
    if let syn::Type::Path(type_path) = field_type {
        if let Some(path_segment) = type_path.path.segments.last() {
            if path_segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &path_segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                        return extract_inner_type(inner_type);
                    }
                }
            }
            return path_segment.ident.to_string();
        }
    }
    panic!("Invalid type")
}
