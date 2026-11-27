fn main() {
    for icon in [
        "icons/icon.ico",
        "icons/32x32.png",
        "icons/128x128.png",
        "icons/128x128@2x.png",
        "icons/icon.icns",
    ] {
        println!("cargo:rerun-if-changed={icon}");
    }
    tauri_build::build();
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        // `tauri_build` embeds the ComCtl32 v6 manifest only into `bin` targets;
        // test binaries that instantiate `tauri::Wry` (e.g. `tauri-specta`
        // binding export) need it too or Windows aborts on `TaskDialogIndirect`.
        println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-tests=/MANIFESTDEPENDENCY:type='win32' \
             name='Microsoft.Windows.Common-Controls' version='6.0.0.0' \
             processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
        );
    }
}
