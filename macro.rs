use proc_macro::TokenStream;
use quote::{format_ident, quote};
use serde_json::Value;
use std::{fs, path::Path};
use syn::{parse::Parse, parse::ParseStream, parse_macro_input, DeriveInput, LitStr, Token, Visibility};

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

fn parse_content(content: &str, format: &str) -> Result<Value, Box<dyn std::error::Error>> {
    match format {
        "json" => Ok(serde_json::from_str(content)?),
        "yaml" | "yml" => Ok(serde_yaml::from_str(content)?),
        "toml" => Ok(toml::from_str(content)?),
        _ => Err(format!("Unsupported format: {}", format).into()),
    }
}

fn generate_field_info(value: &Value, parent_name: &str) -> Vec<(syn::Ident, syn::Type, proc_macro2::TokenStream)> {
    match value {
        Value::Object(map) => map
            .iter()
            .map(|(key, value)| {
                let field_name = format_ident!("{}", key);
                let (field_type, field_value) = generate_field_type(value, &format!("{}{}", parent_name, to_pascal_case(key)));
                (field_name, syn::parse_str(&field_type.to_string()).unwrap(), field_value)
            })
            .collect(),
        _ => vec![],
    }
}

fn generate_field_type(value: &Value, parent_name: &str) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    match value {
        Value::Bool(b) => (quote!(bool), quote!(#b)),
        Value::String(s) => (quote!(String), quote!(#s.to_string())),

        Value::Number(n) => {
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

        Value::Array(arr) => {
            if let Some(first) = arr.first() {
                let (inner_type, _) = generate_field_type(first, parent_name);
                let values = arr.iter().map(|v| {
                    let (_, value) = generate_field_type(v, parent_name);
                    value
                });
                (quote!(Vec<#inner_type>), quote!(vec![#(#values),*]))
            } else {
                (quote!(Vec<Value>), quote!(vec![]))
            }
        }

        Value::Object(obj) => {
            if obj.contains_key("opt_some") {
                let (inner_type, inner_value) = generate_field_type(&obj["opt_some"], parent_name);
                (quote!(Option<#inner_type>), quote!(Some(#inner_value)))
            } else if obj.contains_key("opt_none") {
                let (inner_type, _) = generate_field_type(&obj["opt_none"], parent_name);
                (quote!(Option<#inner_type>), quote!(None))
            } else {
                let nested_name = format_ident!("{}", parent_name);
                (quote!(#nested_name), quote!(#nested_name::default()))
            }
        }

        field => panic!("Cannot use {field} in the struct generator"),
    }
}

fn generate_nested_structs(value: &Value, parent_name: &str, vis: &Visibility) -> Vec<proc_macro2::TokenStream> {
    match value {
        Value::Object(map) => {
            let mut structs = vec![];
            for (key, value) in map {
                if let Value::Object(_) = value {
                    let nested_name = format_ident!("{}{}", parent_name, to_pascal_case(key));
                    let nested_fields = generate_field_info(value, &nested_name.to_string());
                    let nested_field_names = nested_fields.iter().map(|(name, _, _)| name);
                    let nested_field_types = nested_fields.iter().map(|(_, ty, _)| ty);
                    let nested_field_initializers = nested_fields.iter().map(|(name, _, value)| {
                        quote! { #name: #value }
                    });
                    let nested_struct = quote! {
                        #[derive(Debug, Clone, Default, Eq, PartialEq)]
                        #vis struct #nested_name {
                            #(#vis #nested_field_names: #nested_field_types,)*
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
                    structs.extend(generate_nested_structs(value, &nested_name.to_string(), vis));
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
    let vis = &input.vis;

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
    let field_initializers = fields.iter().map(|(name, _, value)| quote!(#name: #value));
    let nested_structs = generate_nested_structs(&parsed_value, &struct_name.to_string(), vis);

    let expanded = quote! {
        #(#nested_structs)*

        #[derive(Debug, Clone, Default, PartialEq)]
        #vis struct #struct_name {
            #(#vis #field_names: #field_types,)*
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
