pub mod events;
mod migrations;
pub mod settings;
pub mod task_filters;
pub mod tasks;
pub mod ui_state;

pub use events::*;
pub use migrations::init_db;
pub use settings::*;
pub use task_filters::*;
pub use tasks::*;
pub use ui_state::*;

pub trait FilterColumnExt {
    fn apply_sql(
        &self,
        builder: &mut sqlx::QueryBuilder<sqlx::Sqlite>,
        op: &str,
        val: &serde_json::Value,
    );
}

#[cfg(test)]
mod tests;
