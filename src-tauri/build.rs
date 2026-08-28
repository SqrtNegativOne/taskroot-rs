use serde::Deserialize;
use std::env;
use std::fs;
use std::path::Path;
use std::fmt::Write;

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

                let default_val_str = setting.default_value.as_ref().map_or_else(
                    || "Default::default()".to_string(),
                    |val| match val {
                        serde_yaml::Value::String(s) => format!("\"{s}\".to_string()"),
                        serde_yaml::Value::Bool(b) => b.to_string(),
                        serde_yaml::Value::Number(n) => n.to_string(),
                        _ => "Default::default()".to_string(),
                    }
                );

                let _ = writeln!(struct_fields, "    pub {}: {rust_type},", setting.id);
                let _ = writeln!(
                    default_impls,
                    "            {}: {default_val_str},",
                    setting.id
                );
            }
        }
    }

    let generated_code = format!(
        r#"
use serde::{{Deserialize, Serialize}};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/AppSettings.generated.ts")]
pub struct AppSettings {{
{struct_fields}
}}

impl Default for AppSettings {{
    fn default() -> Self {{
        Self {{
{default_impls}
        }}
    }}
}}
"#
    );

    let out_dir = env::var_os("OUT_DIR").expect("OUT_DIR not set");
    let dest_path = Path::new(&out_dir).join("settings_generated.rs");
    fs::write(&dest_path, generated_code).expect("Failed to write settings_generated.rs");

    // Convert YAML directly to JSON without the intermediate restrictive struct
    let original_yaml: serde_yaml::Value = serde_yaml::from_str(&yaml_str).expect("Failed to parse original yaml");
    let final_json = serde_json::to_string(&original_yaml).expect("Failed to serialize final json");
    let json_dest_path = Path::new(&out_dir).join("settings_schema.json");
    fs::write(&json_dest_path, final_json).expect("Failed to write json");
}
