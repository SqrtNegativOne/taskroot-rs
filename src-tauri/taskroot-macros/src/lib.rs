use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields};

#[proc_macro_derive(Filterable, attributes(filter))]
pub fn filterable_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;
    let enum_name = syn::Ident::new(&format!("{}FilterColumn", name), name.span());
    let col_def_name = syn::Ident::new(&format!("{}ColumnDef", name), name.span());

    let mut schema_entries = Vec::new();
    let mut enum_variants = Vec::new();


    let Data::Struct(data_struct) = ast.data else {
        return quote! { compile_error!("Filterable can only be derived for structs"); }.into();
    };

    let Fields::Named(fields_named) = data_struct.fields else {
        return quote! { compile_error!("Filterable requires named fields"); }.into();
    };

    for field in fields_named.named {
        let field_name = field.ident.unwrap();
        let field_name_str = field_name.to_string();
        
        // Convert to PascalCase for enum variant
        let mut chars = field_name_str.chars();
        let pascal_name = match chars.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().chain(chars).collect(),
        };
        // Special case: `parent_task` -> `ParentTask` (just basic case mapping for simple toy struct, 
        // wait, we can just use syn::Ident)
        // Actually, let's just use heck crate or simple capitalized:
        let pascal_name = pascal_name.replace("_", ""); // simplified
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
            if attr.path().is_ident("filter") {
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
