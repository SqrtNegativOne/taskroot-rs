//! Thin binary entry point for the headless CLI; the implementation lives in
//! `tauri_app_lib::cli`.

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    match tauri_app_lib::cli::run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
