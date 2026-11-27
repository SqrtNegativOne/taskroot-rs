//! Regenerates `src/lib/bindings/index.generated.ts` from `ipc::specta_builder()`.
//!
//! Kept in an integration test so the unit-test binary does not need to link
//! `tauri::Wry`, while `build.rs` attaches the Windows ComCtl32 v6 manifest to
//! test targets.

use specta_typescript::Typescript;

#[test]
fn exports_typescript_bindings() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../src/lib/bindings/index.generated.ts");

    taskroot_lib::ipc::specta_builder()
        .export(Typescript::default(), path)
        .expect("failed to export TypeScript IPC bindings");
}
