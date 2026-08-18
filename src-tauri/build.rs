use serde::Deserialize;
use std::env;
use std::fs;
use std::path::Path;

#[derive(Deserialize, Debug)]
struct SettingsSchemaYaml {
    tabs: Vec<TabYaml>,
}

#[derive(Deserialize, Debug)]
struct TabYaml {
    #[serde(default)]
    sections: Vec<SectionYaml>,
}

#[derive(Deserialize, Debug)]
struct SectionYaml {
    #[serde(default)]
    settings: Vec<SettingYaml>,
}

#[derive(Deserialize, Debug)]
struct SettingYaml {
    id: String,
    #[serde(rename = "type")]
    setting_type: String,
    #[serde(rename = "defaultValue")]
    default_value: Option<serde_yaml::Value>,
}

fn main() {
    tauri_build::build();

    println!("cargo:rerun-if-changed=settings.yaml");

    let yaml_str = fs::read_to_string("settings.yaml").expect("Failed to read settings.yaml");
    let schema: SettingsSchemaYaml = serde_yaml::from_str(&yaml_str).expect("Failed to parse YAML");

    let mut struct_fields = String::new();
    let mut default_impls = String::new();

    for tab in &schema.tabs {
        for section in &tab.sections {
            for setting in &section.settings {
                if setting.setting_type == "custom" || setting.setting_type == "action" {
                    continue;
                }

                let rust_type = match setting.setting_type.as_str() {
                    "checkbox" => "bool",
                    "number" | "time" => "i32",
                    "select" | "keybinding" | "textarea" => {
                        if let Some(serde_yaml::Value::Number(_)) = setting.default_value {
                            "i32"
                        } else {
                            "String"
                        }
                    }
                    _ => "String",
                };

                let default_val_str = if let Some(val) = &setting.default_value {
                    match val {
                        serde_yaml::Value::String(s) => format!("\"{}\".to_string()", s),
                        serde_yaml::Value::Bool(b) => b.to_string(),
                        serde_yaml::Value::Number(n) => n.to_string(),
                        _ => "Default::default()".to_string(),
                    }
                } else {
                    "Default::default()".to_string()
                };

                struct_fields.push_str(&format!("    pub {}: {},\n", setting.id, rust_type));
                default_impls.push_str(&format!("            {}: {},\n", setting.id, default_val_str));
            }
        }
    }

    let generated_code = format!(
        r#"
use serde::{{Deserialize, Serialize}};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AppSettings {{
{}
}}

impl Default for AppSettings {{
    fn default() -> Self {{
        Self {{
{}
        }}
    }}
}}
"#,
        struct_fields, default_impls
    );

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("settings_generated.rs");
    fs::write(&dest_path, generated_code).unwrap();
    
    // Convert YAML directly to JSON without the intermediate restrictive struct
    let original_yaml: serde_yaml::Value = serde_yaml::from_str(&yaml_str).unwrap();
    let final_json = serde_json::to_string(&original_yaml).unwrap();
    let json_dest_path = Path::new(&out_dir).join("settings_schema.json");
    fs::write(&json_dest_path, final_json).unwrap();
}
