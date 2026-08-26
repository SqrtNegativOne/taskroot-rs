use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use sqlx::SqlitePool;

#[derive(Deserialize, Debug)]
struct GoogleTaskList {
    items: Option<Vec<GoogleTask>>,
}

#[derive(Deserialize, Debug)]
struct GoogleTask {
    id: String,
    title: Option<String>,
    notes: Option<String>,
    due: Option<String>,
    status: Option<String>, // "needsAction" or "completed"
    parent: Option<String>,
    updated: Option<String>,
    deleted: Option<bool>,
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn sync(pool: &SqlitePool, access_token: &str) -> Result<()> {
    let client = Client::new();
    let url = "https://tasks.googleapis.com/tasks/v1/lists/@default/tasks?showCompleted=true&showHidden=true";

    let response: reqwest::Response = client.get(url).bearer_auth(access_token).send().await?;

    if !response.status().is_success() {
        let err = response.text().await?;
        return Err(anyhow::anyhow!("Google Tasks API error: {err}"));
    }

    let task_list: GoogleTaskList = response.json().await?;

    if let Some(tasks) = task_list.items {
        for task in tasks {
            let app_task_id = format!("google_{}", task.id);
            let is_deleted = task.deleted.unwrap_or(false);

            let remote_updated_at = task.updated.clone().unwrap_or_else(
                || chrono::Utc::now().to_rfc3339()
            );

            if let Ok(Some(local_task)) = crate::db::get_task(pool, &app_task_id).await {
                if let Some(local_updated) = &local_task.updated_at {
                    if local_updated > &remote_updated_at {
                        continue;
                    }
                }
            }

            if is_deleted {
                let _ = crate::db::delete_task(pool, app_task_id).await;
                continue;
            }

            let title = task.title.unwrap_or_else(|| "No Title".to_string());

            let status = match task.status.as_deref() {
                Some("completed") => Some(crate::domain::AppTaskStatus::Done),
                _ => Some(crate::domain::AppTaskStatus::Todo), // "needsAction" usually
            };

            let app_task = crate::domain::AppTask {
                id: app_task_id,
                title,
                status,
                priority: None,
                tags: None,
                checklist: None,
                parent_task: task.parent,
                dependencies: None,
                est: None,
                added: Some(chrono::Utc::now().to_rfc3339()),
                canvas_x: None,
                canvas_y: None,
                on_canvas: None,
                remote_id: Some(task.id),
                notes: task.notes,
                tabs: None,
                due: task.due,
                updated_at: Some(remote_updated_at),
                etag: None,
                dirty: Some(false),
            };

            if let Err(e) = crate::db::upsert_task(pool, app_task).await {
                eprintln!("Failed to upsert Google task: {e}");
            }
        }
    }

    Ok(())
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn publish(task: &crate::domain::AppTask, access_token: &str) -> Result<String> {
    let client = Client::new();

    let status = match task.status {
        Some(crate::domain::AppTaskStatus::Done) => "completed",
        _ => "needsAction",
    };

    let google_task = serde_json::json!({
        "title": task.title,
        "notes": task.notes,
        "due": task.due,
        "status": status,
    });

    let (url, method) = task.remote_id.as_ref().map_or_else(
        || {
            (
                "https://tasks.googleapis.com/tasks/v1/lists/@default/tasks".to_string(),
                reqwest::Method::POST,
            )
        },
        |remote_id| {
            (
                format!("https://tasks.googleapis.com/tasks/v1/lists/@default/tasks/{remote_id}"),
                reqwest::Method::PUT,
            )
        },
    );

    let response = client
        .request(method, &url)
        .bearer_auth(access_token)
        .json(&google_task)
        .send()
        .await?;

    if !response.status().is_success() {
        let err = response.text().await?;
        return Err(anyhow::anyhow!("Failed to publish Google Task: {err}"));
    }

    let created: GoogleTask = response.json().await?;
    Ok(created.id)
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn delete(remote_id: &str, access_token: &str) -> Result<()> {
    let client = Client::new();
    let url = format!("https://tasks.googleapis.com/tasks/v1/lists/@default/tasks/{remote_id}");
    let response = client
        .request(reqwest::Method::DELETE, &url)
        .bearer_auth(access_token)
        .send()
        .await?;

    if !response.status().is_success() {
        let err = response.text().await?;
        return Err(anyhow::anyhow!("Failed to delete Google Task: {err}"));
    }
    Ok(())
}
