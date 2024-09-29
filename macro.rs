use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::fs;
use std::path::Path;
use syn::{parse::Parse, parse::ParseStream, parse_macro_input, DeriveInput, LitStr, Token};

struct GenerateArgs {
    file_path: String,
    format: Option<String>,
}

impl Parse for GenerateArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let file_path: LitStr = input.parse()?;
        let format = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            Some(input.parse::<LitStr>()?.value())
        } else {
            None
        };
        Ok(GenerateArgs { file_path: file_path.value(), format })
    }
}

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| word.chars().next().map(|c| c.to_uppercase().collect::<String>() + &word[1..]).unwrap_or_default())
        .collect()
}

fn parse_content(content: &str, format: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match format {
        "json" => Ok(serde_json::from_str(content)?),
        "yaml" | "yml" => Ok(serde_yaml::from_str(content)?),
        "toml" => Ok(toml::from_str(content)?),
        _ => Err(format!("Unsupported format: {}", format).into()),
    }
}

fn generate_field_info(value: &serde_json::Value, parent_name: &str) -> Vec<(syn::Ident, syn::Type, proc_macro2::TokenStream)> {
    match value {
        serde_json::Value::Object(map) => map
            .iter()
            .map(|(key, value)| {
                let field_name = format_ident!("{}", key);
                let (field_type, field_value) = match value {
                    serde_json::Value::Null => (quote!(Option<String>), quote!(None)),
                    serde_json::Value::Bool(b) => (quote!(bool), quote!(#b)),
                    serde_json::Value::Number(n) => {
                        if n.is_i64() {
                            let i = n.as_i64().unwrap();
                            (quote!(i64), quote!(#i))
                        } else if n.is_u64() {
                            let u = n.as_u64().unwrap();
                            (quote!(u64), quote!(#u))
                        } else {
                            let f = n.as_f64().unwrap();
                            (quote!(f64), quote!(#f))
                        }
                    }
                    serde_json::Value::String(s) => (quote!(String), quote!(#s.to_string())),
                    serde_json::Value::Array(arr) => {
                        if let Some(first) = arr.first() {
                            let (inner_type, _) = generate_field_type(first, &format!("{}_{}", parent_name, key));
                            (quote!(Vec<#inner_type>), quote!(vec![]))
                        } else {
                            (quote!(Vec<serde_json::Value>), quote!(vec![]))
                        }
                    }
                    serde_json::Value::Object(_) => {
                        let nested_name = format_ident!("{}{}", parent_name, to_pascal_case(key));
                        (quote!(#nested_name), quote!(#nested_name::new()))
                    }
                };
                (field_name, syn::parse_str(&field_type.to_string()).unwrap(), field_value)
            })
            .collect(),
        _ => vec![],
    }
}

fn generate_field_type(value: &serde_json::Value, parent_name: &str) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    match value {
        serde_json::Value::Null => (quote!(Option<String>), quote!(None)),
        serde_json::Value::Bool(_) => (quote!(bool), quote!(false)),
        serde_json::Value::Number(_) => (quote!(f64), quote!(0.0)),
        serde_json::Value::String(_) => (quote!(String), quote!(String::new())),
        serde_json::Value::Array(arr) => {
            if let Some(first) = arr.first() {
                let (inner_type, _) = generate_field_type(first, parent_name);
                (quote!(Vec<#inner_type>), quote!(vec![]))
            } else {
                (quote!(Vec<serde_json::Value>), quote!(vec![]))
            }
        }
        serde_json::Value::Object(_) => {
            let nested_name = format_ident!("{}", parent_name);
            (quote!(#nested_name), quote!(#nested_name::default()))
        }
    }
}

fn generate_nested_structs(value: &serde_json::Value, parent_name: &str) -> Vec<proc_macro2::TokenStream> {
    match value {
        serde_json::Value::Object(map) => {
            let mut structs = vec![];
            for (key, value) in map {
                if let serde_json::Value::Object(_) = value {
                    let nested_name = format_ident!("{}{}", parent_name, to_pascal_case(key));
                    let nested_fields = generate_field_info(value, &nested_name.to_string());
                    let nested_field_names = nested_fields.iter().map(|(name, _, _)| name);
                    let nested_field_types = nested_fields.iter().map(|(_, ty, _)| ty);
                    let nested_field_initializers = nested_fields.iter().map(|(name, _, value)| {
                        quote! { #name: #value }
                    });
                    let nested_struct = quote! {
                        #[derive(Debug, Clone, Default, Eq, PartialEq)]
                        pub struct #nested_name {
                            #(pub #nested_field_names: #nested_field_types,)*
                        }

                        impl #nested_name {
                            pub fn new() -> Self {
                                Self {
                                    #(#nested_field_initializers,)*
                                }
                            }
                        }
                    };
                    structs.push(nested_struct);
                    structs.extend(generate_nested_structs(value, &nested_name.to_string()));
                }
            }
            structs
        }
        _ => vec![],
    }
}

#[proc_macro_attribute]
pub fn generate(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as GenerateArgs);
    let input = parse_macro_input!(item as DeriveInput);
    let struct_name = &input.ident;

    let file_path = &args.file_path;
    let format = args
        .format
        .unwrap_or_else(|| Path::new(file_path).extension().and_then(|os_str| os_str.to_str()).unwrap_or("").to_string());

    let file_content = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => return syn::Error::new(struct_name.span(), format!("Failed to read file '{}': {}", file_path, e)).to_compile_error().into(),
    };

    let parsed_value = match parse_content(&file_content, &format) {
        Ok(value) => value,
        Err(e) => return syn::Error::new(struct_name.span(), format!("Failed to parse content: {}", e)).to_compile_error().into(),
    };

    let fields = generate_field_info(&parsed_value, &struct_name.to_string());
    let field_names = fields.iter().map(|(name, _, _)| name);
    let field_types = fields.iter().map(|(_, ty, _)| ty);
    let field_initializers = fields.iter().map(|(name, _, value)| {
        quote! { #name: #value }
    });

    let nested_structs = generate_nested_structs(&parsed_value, &struct_name.to_string());

    let expanded = quote! {
        #(#nested_structs)*

        #[derive(Debug, Clone, Default, Eq, PartialEq)]
        pub struct #struct_name {
            #(pub #field_names: #field_types,)*
        }

        impl #struct_name {
            pub fn new() -> Self {
                Self {
                    #(#field_initializers,)*
                }
            }

            pub fn is_empty(&self) -> bool {
                self == &Self::default()
            }
        }
    };

    TokenStream::from(expanded)
}
