use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Lit};

#[proc_macro_derive(Model, attributes(model))]
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

    let mut schema_fields = Vec::new();
    let mut create_args = Vec::new();
    let mut update_args = Vec::new();

    let mut the_primary_key = quote! {};

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = match &field.ty {
            syn::Type::Path(type_path) => type_path.path.segments.last().unwrap().ident.to_string(),
            _ => panic!("Unsupported field type"),
        };

        let mut is_nullable = true;
        let mut is_primary_key = false;
        let mut is_auto = false;
        let mut is_unique = false;
        let mut is_default = false;
        let mut size = None;
        let mut default = quote! {};
        let mut foreign_key = quote! {};

        for attr in &field.attrs {
            if attr.path.is_ident("model") {
                let meta = attr.parse_meta().unwrap();
                if let syn::Meta::List(ref list) = meta {
                    for nested in &list.nested {
                        if let syn::NestedMeta::Meta(syn::Meta::NameValue(ref nv)) = nested {
                            if nv.path.is_ident("primary_key") {
                                if let Lit::Bool(ref lit) = nv.lit {
                                    the_primary_key = quote! { #field_name.clone() };
                                    is_primary_key = lit.value;
                                }
                            } else if nv.path.is_ident("auto") {
                                if let Lit::Bool(ref lit) = nv.lit {
                                    is_auto = lit.value;
                                }
                            } else if nv.path.is_ident("size") {
                                if let Lit::Int(ref lit) = nv.lit {
                                    size = Some(lit.clone());
                                }
                            } else if nv.path.is_ident("unique") {
                                if let Lit::Bool(ref lit) = nv.lit {
                                    is_unique = lit.value;
                                }
                            } else if nv.path.is_ident("null") {
                                if let Lit::Bool(ref lit) = nv.lit {
                                    is_nullable = lit.value;
                                }
                            } else if nv.path.is_ident("default") {
                                is_default = true;
                                if let Lit::Str(ref str) = nv.lit {
                                    default = if str.value() == "now" {
                                        if field_type == "Date" {
                                            quote! { default current_date}
                                        } else if field_type == "DateTime" {
                                            quote! { default current_timestamp}
                                        } else {
                                            panic!("'now' is work only with Date or DateTime");
                                        }
                                    } else {
                                        let str = format!("'{str}'", str = str.value());
                                        quote! { default #str }
                                    }
                                } else if let Lit::Bool(ref bool) = nv.lit {
                                    default = if bool.value {
                                        quote! {default 1}
                                    } else {
                                        quote! {default 0}
                                    };
                                } else if let Lit::Int(ref int) = nv.lit {
                                    default = quote! { default #int }
                                }
                            } else if nv.path.is_ident("foreign_key") {
                                if let Lit::Str(ref lit) = nv.lit {
                                    let fk = lit.value();
                                    let foreign_key_parts: Vec<&str> = fk.split('.').collect();
                                    if foreign_key_parts.len() != 2 {
                                        panic!("Invalid foreign key");
                                    }
                                    let foreign_key_table = foreign_key_parts[0];
                                    let foreign_key_field = foreign_key_parts[1];

                                    foreign_key = quote! {
                                         references #foreign_key_table(#foreign_key_field)
                                    };
                                }
                            }
                        }
                    }
                }
            }
        }

        let field_schema = {
            let base_type = match field_type.as_str() {
                "Serial" => quote! { serial },
                "Integer" => quote! { integer },
                "String" => {
                    if let Some(size) = size {
                        quote! {varchar(#size)}
                    } else {
                        quote! {varchar(255)}
                    }
                }
                "Float" => quote! { float },
                "Text" => quote! { text },
                "Date" => quote! { varchar(10) },
                "Boolean" | "bool" => quote! { integer },
                "DateTime" => quote! { varchar(40) },
                p_type => panic!("{}", p_type),
            };

            let primary_key = if is_primary_key {
                let auto = if is_auto {
                    quote! { autoincrement }
                } else if field_type.as_str() == "Serial" {
                    quote! {}
                } else {
                    create_args.push(quote! { #field_name });
                    quote! {}
                };
                quote! { primary key #auto}
            } else {
                create_args.push(quote! { #field_name });
                update_args.push(quote! { #field_name });
                quote! {}
            };

            if is_default {
                create_args.pop();
            }

            let nullable = if is_nullable {
                quote! {}
            } else {
                quote! {not null}
            };
            let unique = if is_unique {
                quote! { unique }
            } else {
                quote! {}
            };

            quote! { #field_name #base_type #primary_key #unique #default #nullable #foreign_key }
        };

        schema_fields.push(field_schema);
    }

    let primary_key = {
        let pk = the_primary_key.to_string().replace(".clone()", "");
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

    let create = quote! {
        async fn save(&self, conn: &Connection) -> bool {
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
        async fn update(&self, conn: &Connection) -> bool {
            Self::set(
                self.#the_primary_key,
                kwargs!(
                    #(#update_args = self.#update_args),*
                ),
                conn,
            )
            .await
        }
    };

    let delete = {
        let query =
            format!("delete from {name} where {the_primary_key}=?1;").replace(".clone()", "");
        quote! {
            async fn delete(&self, conn: &Connection) -> bool {
                sqlx::query(&#query.replace("?", rusql_alchemy::PLACEHOLDER).replace("$", rusql_alchemy::PLACEHOLDER))
                    .bind(self.#the_primary_key)
                    .execute(conn)
                    .await
                    .is_ok()
            }
        }
    };

    let expanded = quote! {
        #[async_trait]
        impl Model<sqlx::any::AnyRow> for #name {
            const NAME: &'static str = stringify!(#name);
            #schema
            #primary_key
            #create
            #update
            #delete
        }
    };

    TokenStream::from(expanded)
}
