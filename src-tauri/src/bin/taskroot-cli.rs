//! Thin binary entry point for the headless CLI; the implementation lives in
//! `taskroot_lib::cli`.

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    match taskroot_lib::cli::run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
