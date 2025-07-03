use quote::{format_ident, quote};
use std::collections::HashMap;
use std::fs;
use syn::{parse_macro_input, LitStr};

struct Args {
    locales_path: String,
}

impl Args {
    fn consume_path(&mut self, input: syn::parse::ParseStream) -> syn::parse::Result<()> {
        let path = input.parse::<LitStr>()?;
        self.locales_path = path.value();

        Ok(())
    }
}

impl syn::parse::Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self> {
        let lookahead = input.lookahead1();

        let mut result = Self { locales_path: String::from("locales") };

        if lookahead.peek(LitStr) {
            result.consume_path(input)?;
        }
        Ok(result)
    }
}

#[proc_macro]
pub fn i18n(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(input as Args);

    // CARGO_MANIFEST_DIR is current build directory
    let cargo_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is empty");
    let current_dir = std::path::PathBuf::from(cargo_dir);
    let locales_path = current_dir.join(&args.locales_path);
    if !locales_path.exists() {
        panic!("cargo:i18n-error=path not exists: {:?}", locales_path);
    }
    if !locales_path.is_dir() {
        panic!("cargo:i18n-error=path not a dir: {:?}", locales_path);
    }
    let data = load_locales(&locales_path);
    match data {
        Err(e) => {
            panic!("cargo:i18n-error=parse local: {}", e);
        }
        Ok(data) => {
            let code = generate_code(data, args);
            code.into()
        }
    }
}

fn load_locales(path: &std::path::PathBuf) -> anyhow::Result<Vec<serde_json::Value>> {
    let mut v = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            let s = load_locales(&entry_path)?;
            v.extend(s);
        }
        if entry_path.is_file() {
            let path = entry_path.display().to_string();
            let (_, ext) = path
                .rsplit_once(".")
                .ok_or(anyhow::Error::msg("cargo:i18n-error=path not found ext:"))?;
            let content = fs::read_to_string(entry_path.clone())?;
            let result = match ext {
                "yml" | "yaml" => serde_yaml::from_str::<serde_json::Value>(&content)
                    .map_err(|err| format!("Invalid YAML format, {}", err)),
                "json" => serde_json::from_str::<serde_json::Value>(&content)
                    .map_err(|err| format!("Invalid JSON format, {}", err)),
                "toml" => toml::from_str::<serde_json::Value>(&content)
                    .map_err(|err| format!("Invalid TOML format, {}", err)),
                _ => Err("Invalid file extension".into()),
            };
            v.push(result.map_err(anyhow::Error::msg)?);
        }
    }
    Ok(v)
}

fn parse_locales(locales: Vec<serde_json::Value>) -> anyhow::Result<HashMap<String, Vec<String>>> {
    let mut hash = HashMap::new();
    for locale in locales {
        parse_file_locale(locale, &mut hash)?;
    }
    Ok(hash)
}

fn parse_file_locale(
    value: serde_json::Value,
    map: &mut HashMap<String, Vec<String>>,
) -> anyhow::Result<()> {
    let mut flat_map = HashMap::new();
    flatten_json(&value, String::new(), &mut flat_map);
    for (key, value) in flat_map {
        if let Some((key, _)) = key.rsplit_once(".") {
            let placeholders = extract_placeholders(&value);
            map.insert(key.to_string(), placeholders);
        }
    }
    Ok(())
}

fn extract_placeholders(input: &str) -> Vec<String> {
    let re = regex::Regex::new(r"%\{([^}]*)}").unwrap();

    let mut placeholders = Vec::new();
    for cap in re.captures_iter(input) {
        if let Some(matched) = cap.get(1) {
            placeholders.push(matched.as_str().trim().to_string());
        }
    }

    placeholders
}

fn flatten_json(value: &serde_json::Value, prefix: String, flat_map: &mut HashMap<String, String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let new_prefix =
                    if prefix.is_empty() { k.clone() } else { format!("{}.{}", prefix, k) };
                flatten_json(v, new_prefix, flat_map);
            }
        }
        _ => {
            flat_map.insert(prefix, value_to_string(value));
        }
    }
}

fn value_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => format!("{:?}", arr),
        serde_json::Value::Object(map) => format!("{:?}", map),
    }
}

fn generate_code(values: Vec<serde_json::Value>, args: Args) -> proc_macro2::TokenStream {
    let map = parse_locales(values).expect("parse error");
    let path = args.locales_path;
    let v: Vec<_> = map
        .into_iter()
        .map(|(k, v)| {
            let v: Vec<_> = v.iter().map(|v| format_ident!("{}", v)).collect();
            if v.is_empty() {
                quote! {
                    (#k) => {
                        rust_i18n::t!(#k, locale=$crate::locale())
                    }
                }
            } else {
                quote! {
                    (#k, #(#v = $#v:expr),*) => {
                        rust_i18n::t!(#k, locale=$crate::locale(), #(#v = $#v),*)
                    }
                }
            }
        })
        .collect();

    quote! {
            rust_i18n::i18n!(#path);

            #[macro_export]
            macro_rules! __my_translation {
                #(#v; )*
            }
            pub use __my_translation as _my_translation;
    }
}
