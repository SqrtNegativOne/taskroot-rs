use crate::domain::{AppTask, AppTaskFilter};
use sqlx::SqlitePool;
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

    let schema = AppTask::get_schema();

    for f in &filters {
        if let (Some(col_id), Some(val)) = (&f.column, &f.value) {
            let op = f.operator.as_deref().unwrap_or("is");
            apply_sql(&mut query_builder, col_id, op, val, &schema);
        }
    }

    push_search_clause(&mut query_builder, &query_text);
    push_sort_clause(&mut query_builder, &sort, &schema);

    let tasks = query_builder
        .build_query_as::<AppTask>()
        .fetch_all(pool)
        .await?;
    Ok(tasks)
}

fn apply_sql(
    builder: &mut sqlx::QueryBuilder<sqlx::Sqlite>,
    col_id: &crate::domain::AppTaskFilterColumn,
    op: &str,
    val: &serde_json::Value,
    schema: &[crate::domain::AppTaskColumnDef],
) {
    let Some(def) = schema.iter().find(|c| &c.id == col_id) else {
        return;
    };

    let is_not = op == "is not";
    let values = val
        .as_array()
        .map_or_else(|| vec![val.clone()], std::clone::Clone::clone);
    if values.is_empty() {
        return;
    }

    match &def.filter_type {
        crate::domain::FilterType::Relation(rel) => {
            if rel == "task_tags" {
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
        _ => {
            // Text, Number, Enum
            builder.push(if is_not {
                format!(" AND {} NOT IN (", def.db_col)
            } else {
                format!(" AND {} IN (", def.db_col)
            });
            let mut separated = builder.separated(", ");
            for v in &values {
                if let Some(n) = v.as_i64() {
                    separated.push_bind(i32::try_from(n).unwrap_or(0));
                } else if let Some(s) = v.as_str() {
                    if let crate::domain::FilterType::Number = &def.filter_type {
                        separated.push_bind(s.parse::<i32>().unwrap_or(-1));
                    } else {
                        separated.push_bind(s);
                    }
                }
            }
            builder.push(")");
        }
    }
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

fn push_sort_clause(query_builder: &mut sqlx::QueryBuilder<sqlx::Sqlite>, sort: &str, schema: &[crate::domain::AppTaskColumnDef]) {
    query_builder.push(" ORDER BY ");
    let sort_enum = serde_json::from_value::<crate::domain::AppTaskFilterColumn>(serde_json::json!(sort)).ok();
    if let Some(sort_enum) = sort_enum {
        if let Some(def) = schema.iter().find(|c| c.id == sort_enum && c.sortable) {
            query_builder.push(&def.db_col);
            return;
        }
    }
    query_builder.push("t.id ASC");
}
