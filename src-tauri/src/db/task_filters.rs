use super::FilterColumnExt;
use crate::domain::{AppTask, AppTaskFilter, TaskFilterColumn};
use sqlx::SqlitePool;

impl FilterColumnExt for TaskFilterColumn {
    fn apply_sql(
        &self,
        builder: &mut sqlx::QueryBuilder<sqlx::Sqlite>,
        op: &str,
        val: &serde_json::Value,
    ) {
        let is_not = op == "is not";
        let values = val
            .as_array()
            .map_or_else(|| vec![val.clone()], std::clone::Clone::clone);
        if values.is_empty() {
            return;
        }

        match self {
            Self::Status => {
                builder.push(if is_not {
                    " AND status NOT IN ("
                } else {
                    " AND status IN ("
                });
                let mut separated = builder.separated(", ");
                for v in &values {
                    if let Some(s) = v.as_str() {
                        separated.push_bind(s);
                    } else {
                        separated.push_bind(v.to_string());
                    }
                }
                builder.push(")");
            }
            Self::Priority => {
                builder.push(if is_not {
                    " AND priority NOT IN ("
                } else {
                    " AND priority IN ("
                });
                let mut separated = builder.separated(", ");
                for v in &values {
                    if let Some(n) = v.as_i64() {
                        separated.push_bind(i32::try_from(n).unwrap_or(0));
                    } else if let Some(s) = v.as_str() {
                        separated.push_bind(s.parse::<i32>().unwrap_or(-1));
                    }
                }
                builder.push(")");
            }
            Self::Tag => {
                builder.push(" AND (");
                for (i, v) in values.iter().enumerate() {
                    if i > 0 {
                        builder.push(if is_not { " AND " } else { " OR " });
                    }
                    let vs = v.as_str().unwrap_or("");
                    let quantifier = if is_not { "NOT IN" } else { "IN" };
                    builder.push(format!(
                        "t.id {quantifier} (SELECT task_id FROM task_tags tt \
                         JOIN tags tg ON tt.tag_id = tg.id WHERE tg.name LIKE "
                    ));
                    builder.push_bind(format!("%{vs}%"));
                    builder.push(")");
                }
                builder.push(")");
            }
        }
    }
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn get_filtered_tasks(
    pool: &SqlitePool,
    filters: Vec<AppTaskFilter>,
    sort: String,
    query_text: String,
) -> Result<Vec<AppTask>, sqlx::Error> {
    let mut query_builder: sqlx::QueryBuilder<sqlx::Sqlite> =
        sqlx::QueryBuilder::new(super::tasks::task_select_sql!(" WHERE 1=1"));

    for f in &filters {
        if let (Some(col), Some(val)) = (&f.column, &f.value) {
            let op = f.operator.as_deref().unwrap_or("is");
            col.apply_sql(&mut query_builder, op, val);
        }
    }

    push_search_clause(&mut query_builder, &query_text);
    push_sort_clause(&mut query_builder, &sort);

    let tasks = query_builder
        .build_query_as::<AppTask>()
        .fetch_all(pool)
        .await?;
    Ok(tasks)
}

fn push_search_clause(query_builder: &mut sqlx::QueryBuilder<sqlx::Sqlite>, query_text: &str) {
    if !query_text.trim().is_empty() {
        let q = format!("%{}%", query_text.trim().to_lowercase());
        query_builder.push(" AND (LOWER(t.title) LIKE ");
        query_builder.push_bind(q.clone());
        query_builder.push(" OR t.id IN (SELECT task_id FROM task_tags tt JOIN tags tg ON tt.tag_id = tg.id WHERE LOWER(tg.name) LIKE ");
        query_builder.push_bind(q);
        query_builder.push("))");
    }
}

fn push_sort_clause(query_builder: &mut sqlx::QueryBuilder<sqlx::Sqlite>, sort: &str) {
    query_builder.push(" ORDER BY ");
    match sort {
        "priority" => query_builder.push("COALESCE(priority, 0) DESC"),
        "due" => query_builder.push("COALESCE(due, '9999') ASC"),
        "title" => query_builder.push("title ASC"),
        _ => query_builder.push("id ASC"),
    };
}
