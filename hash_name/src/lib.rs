use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Field, Fields};

#[proc_macro_derive(HashNames, attributes(HashNames))]
pub fn field_names_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let fields = if let syn::Data::Struct(data) = input.data {
        data.fields
    } else {
        panic!("HashNames can only be used with structs");
    };

    // skip #[FieldNames(skip)]
    let field_names = fields.iter().filter_map(|field| {
        if has_skip_attribute(field) {
            None
        } else {
            let field_name = field.ident.as_ref().unwrap();
            let field_str = field_name.to_string();
            Some(quote! {
                pub fn #field_name() -> &'static str {
                    #field_str
                }
            })
        }
    });

    let expanded = quote! {
        impl #name {
            #(#field_names)*
        }
    };

    TokenStream::from(expanded)
}

fn has_skip_attribute(field: &Field) -> bool {
    let mut is_skip = false;
    for attr in &field.attrs {
        // 检查路径是否为 "FieldNames"
        if attr.path().is_ident("HashNames") {
            // 解析属性的内容
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("skip") {
                    is_skip = true;
                }
                Ok(())
            })
            .unwrap();
        }
    }
    is_skip
}

#[proc_macro_derive(RedisOps)]
pub fn redis_ops_derive(input: TokenStream) -> TokenStream {
    // 解析输入的 TokenStream
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = input.ident;

    // 获取结构体字段
    let fields = if let Data::Struct(data) = input.data {
        if let Fields::Named(fields) = data.fields {
            fields.named
        } else {
            unimplemented!("Only named fields are supported");
        }
    } else {
        unimplemented!("Only structs are supported");
    };

    let from_redis_value = fields.iter().map(|field| {
        let name = &field.ident;
        let field_name_str = name.as_ref().unwrap().to_string();
        quote! {
            #name: redis::FromRedisValue::from_redis_value(map.remove(#field_name_str).unwrap_or(&redis::Value::Nil) )?
        }
    });

    let to_redis_args = fields.iter().map(|field| {
        let name = &field.ident;
        let field_name_str = name.as_ref().unwrap().to_string();
        quote! {
            match self.#name.to_redis_args() {
                args if args.len() == 1 => {
                    out.write_arg_fmt(#field_name_str);
                    out.write_arg(&args[0]);
                }
                _ => {}
            }
        }
    });

    let expanded = quote! {
        impl redis::FromRedisValue for crate::MyOption<#struct_name> {
            fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Self> {
                match v {
                    redis::Value::Array(arr) if arr.len() % 2 == 0 => {
                        let len = arr.len();
                        if len == 0 || len % 2 != 0 {
                            return Ok(crate::MyOption::None);
                        }
                        let mut map = std::collections::HashMap::new();
                        for values in arr.chunks(2) {
                            let filed : String = redis::from_redis_value(&values[0])?;
                            map.insert(filed, &values[1]);
                        }
                        Ok(crate::MyOption::Some(#struct_name {
                            #(#from_redis_value),*
                        }))
                    }
                     _ => Err(redis::RedisError::from((redis::ErrorKind::TypeError, "not supported value type")))
                }
            }
        }

        impl redis::ToRedisArgs for #struct_name {
            fn write_redis_args<W>(&self, out: &mut W)
            where
                W: ?Sized + redis::RedisWrite,
            {
                        #(#to_redis_args)*
            }
        }
    };

    TokenStream::from(expanded)
}
