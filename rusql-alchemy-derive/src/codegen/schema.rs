use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

fn generate_base_type(field_type: &str, option_size: Option<usize>) -> TokenStream {
    match field_type {
        "Text" => quote! { text },
        "Float" => quote! { float },
        "Serial" => quote! { serial },
        "Integer" => quote! { integer },
        "Boolean" => quote! { integer },
        "Date" => quote! { varchar(10) },
        "DateTime" => quote! { varchar(40) },
        "String" => match option_size {
            Some(size) => {
                let size = size.to_string();
                quote! { varchar(#size) }
            }
            None => quote! { varchar(255) },
        },
        ty => panic!(
            r#"Unexpected field type: '{ty}'
            Expected one of: 'Serial', 'Integer', 'String', 'Float', 'Text', 'Date', 'Boolean', 'DateTime'.
            Please check the field type
            "#
        ),
    }
}

fn generate_pk(
    field_name: &Ident,
    field_type: &str,
    is_auto: bool,
    create_args: &mut Vec<TokenStream>,
) -> TokenStream {
    let auto = match (is_auto, field_type) {
        (true, _) => quote! { autoincrement },
        (false, "Serial") => quote! {},
        (false, _) => {
            create_args.push(quote! { #field_name });
            quote! {}
        }
    };
    quote! { primary key #auto }
}

fn generate_default_schema(option_default: Option<TokenStream>, field_type: &str) -> TokenStream {
    match option_default {
        Some(default_value) => {
            let default_value = default_value.to_string().replace('"', "");
            if default_value == "now" {
                match field_type {
                    "Date" => quote! { default current_date },
                    "DateTime" => quote! { default current_timestamp },
                    _ => panic!("'now' is work only with Date or DateTime"),
                }
            } else if field_type == "Boolean" {
                match default_value.as_str() {
                    "true" => quote! { default 1 },
                    "false" => quote! { default 0 },
                    _ => panic!("'Boolean' type allow only 'true' or 'false' value"),
                }
            } else {
                quote! { default #default_value }
            }
        }
        None => quote! {},
    }
}

#[allow(clippy::too_many_arguments)]
pub fn generate_field_schema(
    field_name: &Ident,
    field_type: &str,
    is_pk: bool,
    is_auto: bool,
    is_nullable: bool,
    is_unique: bool,
    foreign_key: TokenStream,
    option_size: Option<usize>,
    option_default: Option<TokenStream>,
    create_args: &mut Vec<TokenStream>,
    update_args: &mut Vec<TokenStream>,
) -> TokenStream {
    let base_type = generate_base_type(field_type, option_size);

    let primary_key = if is_pk {
        generate_pk(field_name, field_type, is_auto, create_args)
    } else {
        create_args.push(quote! { #field_name });
        update_args.push(quote! { #field_name });
        quote! {}
    };

    let nullable = match is_nullable {
        true => quote! {},
        false => quote! { not null },
    };

    let unique = match is_unique {
        true => quote! { unique },
        false => quote! {},
    };

    let default = generate_default_schema(option_default, field_type);

    quote! { #field_name #base_type #primary_key #unique #default #nullable #foreign_key}
}
