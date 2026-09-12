use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Lit, LitInt, LitStr};

#[proc_macro_derive(Queryable, attributes(query))]
pub fn queryable_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;
    let enum_name = syn::Ident::new(&format!("{}FilterColumn", name), name.span());
    let col_def_name = syn::Ident::new(&format!("{}ColumnDef", name), name.span());
    let filter_name = syn::Ident::new(&format!("{}Filter", name), name.span());
    let sort_name = syn::Ident::new(&format!("{}Sort", name), name.span());

    let mut schema_entries = Vec::new();
    let mut enum_variants = Vec::new();

    let Data::Struct(data_struct) = ast.data else {
        return quote! { compile_error!("Queryable can only be derived for structs"); }.into();
    };

    let Fields::Named(fields_named) = data_struct.fields else {
        return quote! { compile_error!("Queryable requires named fields"); }.into();
    };

    for field in fields_named.named {
        let field_name = field.ident.unwrap();
        let field_name_str = field_name.to_string();
        
        let mut chars = field_name_str.chars();
        let pascal_name = match chars.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().chain(chars).collect(),
        };
        let pascal_name = pascal_name.replace("_", "");
        let variant_ident = syn::Ident::new(&pascal_name, field_name.span());

        let mut is_sortable = false;
        let mut is_filterable = false;
        let mut db_col = field_name_str.clone();
        let mut filter_type = quote! { crate::domain::FilterType::Text };
        let mut label = field_name_str.clone();
        if let Some(r) = label.get_mut(0..1) {
            r.make_ascii_uppercase();
        }

        for attr in field.attrs {
            if attr.path().is_ident("query") {
                is_filterable = true;
                
                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("sortable") {
                        is_sortable = true;
                    } else if meta.path.is_ident("db_col") {
                        let value = meta.value()?;
                        let s: syn::LitStr = value.parse()?;
                        db_col = s.value();
                    } else if meta.path.is_ident("label") {
                        let value = meta.value()?;
                        let s: syn::LitStr = value.parse()?;
                        label = s.value();
                    } else if meta.path.is_ident("filter_type") {
                        let value = meta.value()?;
                        let s: syn::LitStr = value.parse()?;
                        let ft = s.value();
                        if ft == "number" {
                            filter_type = quote! { crate::domain::FilterType::Number };
                        } else if ft.starts_with("enum:") {
                            let enum_name = syn::Ident::new(&ft[5..], proc_macro2::Span::call_site());
                            filter_type = quote! { crate::domain::FilterType::Enum(#enum_name::all_values()) };
                        } else if ft.starts_with("relation:") {
                            let rel = &ft[9..];
                            filter_type = quote! { crate::domain::FilterType::Relation(#rel.to_string()) };
                        }
                    }
                    Ok(())
                });
            }
        }

        if is_filterable || is_sortable {
            enum_variants.push(quote! {
                #variant_ident
            });

            schema_entries.push(quote! {
                #col_def_name {
                    id: #enum_name::#variant_ident,
                    label: #label.to_string(),
                    db_col: #db_col.to_string(),
                    filter_type: #filter_type,
                    sortable: #is_sortable,
                }
            });
        }
    }

    let file_name = format!("../../src/lib/bindings/{}.generated.ts", enum_name);
    let col_def_file = format!("../../src/lib/bindings/{}.generated.ts", col_def_name);
    let filter_file = format!("../../src/lib/bindings/{}.generated.ts", filter_name);
    let sort_file = format!("../../src/lib/bindings/{}.generated.ts", sort_name);
    
    let gen = quote! {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, ts_rs::TS)]
        #[ts(export, export_to = #file_name)]
        #[serde(rename_all = "camelCase")]
        pub enum #enum_name {
            #(#enum_variants),*
        }

        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
        #[ts(export, export_to = #col_def_file)]
        pub struct #col_def_name {
            pub id: #enum_name,
            pub label: String,
            pub db_col: String,
            pub filter_type: crate::domain::FilterType,
            pub sortable: bool,
        }

        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
        #[ts(export, export_to = #filter_file)]
        pub struct #filter_name {
            pub column: Option<#enum_name>,
            pub operator: Option<crate::domain::FilterOperator>,
            #[ts(type = "unknown")]
            pub value: Option<serde_json::Value>,
        }

        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
        #[ts(export, export_to = #sort_file)]
        pub struct #sort_name {
            pub column: Option<#enum_name>,
            pub direction: Option<crate::domain::SortDirection>,
        }

        impl #name {
            pub fn get_schema() -> Vec<#col_def_name> {
                vec![
                    #(#schema_entries),*
                ]
            }
        }
    };
    
    gen.into()
}

struct SettingOptionTokens {
    value: TokenStream2,
    label: String,
}

/// Derives `AppSettings::setting_metadata()` from field-adjacent `#[setting(..)]`
/// attributes, so a setting's label, keywords, control type, options, bounds and
/// section are declared once next to the field that stores it.
///
/// Every named field must carry at least `label`, `kind` and `section`; a missing
/// attribute is a compile error rather than a silently absent schema entry.
#[proc_macro_derive(SettingsMeta, attributes(setting))]
pub fn settings_meta_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    let Data::Struct(data_struct) = ast.data else {
        return quote! { compile_error!("SettingsMeta can only be derived for structs"); }.into();
    };

    let Fields::Named(fields_named) = data_struct.fields else {
        return quote! { compile_error!("SettingsMeta requires named fields"); }.into();
    };

    let mut entries = Vec::new();
    let mut errors = Vec::new();

    for field in fields_named.named {
        let Some(field_name) = field.ident else { continue };
        let mut label: Option<String> = None;
        let mut kind: Option<String> = None;
        let mut section: Option<String> = None;
        let mut description: Option<String> = None;
        let mut keywords: Vec<String> = Vec::new();
        let mut options: Vec<SettingOptionTokens> = Vec::new();
        let mut min: Option<LitInt> = None;
        let mut max: Option<LitInt> = None;
        let mut danger = false;

        for attr in &field.attrs {
            if !attr.path().is_ident("setting") {
                continue;
            }

            let parsed = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("label") {
                    label = Some(meta.value()?.parse::<LitStr>()?.value());
                } else if meta.path.is_ident("kind") {
                    kind = Some(meta.value()?.parse::<LitStr>()?.value());
                } else if meta.path.is_ident("section") {
                    section = Some(meta.value()?.parse::<LitStr>()?.value());
                } else if meta.path.is_ident("description") {
                    description = Some(meta.value()?.parse::<LitStr>()?.value());
                } else if meta.path.is_ident("min") {
                    min = Some(meta.value()?.parse::<LitInt>()?);
                } else if meta.path.is_ident("max") {
                    max = Some(meta.value()?.parse::<LitInt>()?);
                } else if meta.path.is_ident("danger") {
                    danger = true;
                } else if meta.path.is_ident("keywords") {
                    let value = meta.value()?;
                    let array: syn::ExprArray = value.parse()?;
                    for element in array.elems {
                        let syn::Expr::Lit(syn::ExprLit { lit: Lit::Str(text), .. }) = element else {
                            return Err(meta.error("keywords entries must be string literals"));
                        };
                        keywords.push(text.value());
                    }
                } else if meta.path.is_ident("options") {
                    let value = meta.value()?;
                    let array: syn::ExprArray = value.parse()?;
                    for element in array.elems {
                        options.push(parse_setting_option(&meta, element)?);
                    }
                } else {
                    return Err(meta.error(
                        "unknown setting attribute; expected label, kind, section, \
                         description, keywords, options, min, max or danger",
                    ));
                }
                Ok(())
            });

            if let Err(error) = parsed {
                errors.push(error.to_compile_error());
            }
        }

        let (Some(label), Some(kind), Some(section)) = (label, kind, section) else {
            errors.push(syn::Error::new_spanned(
                &field_name,
                "every SettingsMeta field needs #[setting(label = ..., kind = ..., section = ...)]",
            )
            .to_compile_error());
            continue;
        };

        let id = field_name.to_string();
        let description = match description {
            Some(text) => quote! { Some(#text.to_string()) },
            None => quote! { None },
        };
        let keyword_tokens: Vec<TokenStream2> = keywords
            .iter()
            .map(|keyword| quote! { #keyword.to_string() })
            .collect();
        let options = if options.is_empty() {
            quote! { None }
        } else {
            let values = options.iter().map(|option| {
                let value = &option.value;
                let label = &option.label;
                quote! {
                    crate::settings::SettingOption {
                        value: #value,
                        label: #label.to_string(),
                    }
                }
            });
            quote! { Some(vec![ #(#values),* ]) }
        };
        let min = match min {
            Some(value) => quote! { Some(#value) },
            None => quote! { None },
        };
        let max = match max {
            Some(value) => quote! { Some(#value) },
            None => quote! { None },
        };
        let danger = if danger {
            quote! { Some(true) }
        } else {
            quote! { None }
        };

        entries.push(quote! {
            crate::settings::SettingMeta {
                id: #id.to_string(),
                label: #label.to_string(),
                description: #description,
                keywords: vec![ #(#keyword_tokens),* ],
                kind: #kind.to_string(),
                options: #options,
                min: #min,
                max: #max,
                default_value: Some(
                    serde_json::to_value(&__defaults.#field_name)
                        .unwrap_or(serde_json::Value::Null),
                ),
                danger: #danger,
                section: #section,
            }
        });
    }

    if !errors.is_empty() {
        return quote! { #(#errors)* }.into();
    }

    let generated = quote! {
        impl #name {
            /// Presentation metadata for every persisted setting, in field order.
            pub fn setting_metadata() -> Vec<crate::settings::SettingMeta> {
                let __defaults = <#name as ::core::default::Default>::default();
                vec![ #(#entries),* ]
            }
        }
    };

    generated.into()
}

fn parse_setting_option(
    meta: &syn::meta::ParseNestedMeta,
    element: syn::Expr,
) -> syn::Result<SettingOptionTokens> {
    let syn::Expr::Tuple(tuple) = element else {
        return Err(meta.error("options entries must be (value, label) tuples"));
    };

    let mut elems = tuple.elems.into_iter();
    let (Some(value_expr), Some(label_expr)) = (elems.next(), elems.next()) else {
        return Err(meta.error("options entries must be (value, label) tuples"));
    };
    if elems.next().is_some() {
        return Err(meta.error("options entries must be (value, label) tuples"));
    }

    let label = match label_expr {
        syn::Expr::Lit(syn::ExprLit { lit: Lit::Str(text), .. }) => text.value(),
        other => {
            return Err(syn::Error::new_spanned(
                other,
                "option label must be a string literal",
            ));
        }
    };

    let value = match value_expr {
        syn::Expr::Lit(syn::ExprLit { lit: Lit::Str(text), .. }) => {
            quote! { serde_json::json!(#text) }
        }
        syn::Expr::Lit(syn::ExprLit { lit: Lit::Int(number), .. }) => {
            quote! { serde_json::json!(#number) }
        }
        other => {
            return Err(syn::Error::new_spanned(
                other,
                "option value must be a string or integer literal",
            ));
        }
    };

    Ok(SettingOptionTokens { value, label })
}
