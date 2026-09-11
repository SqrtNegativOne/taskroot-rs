use color_eyre::Result;
use reqwest::Client;
use serde::Deserialize;
use sqlx::SqlitePool;

const TASKS_BASE: &str = "https://tasks.googleapis.com/tasks/v1";

#[derive(Deserialize, Debug)]
struct GoogleTaskLists {
    items: Option<Vec<GoogleTaskListEntry>>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Deserialize, Debug)]
struct GoogleTaskListEntry {
    id: String,
}

#[derive(Deserialize, Debug)]
struct GoogleTaskList {
    items: Option<Vec<GoogleTask>>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
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

/// Sync every task list, using the deleted-aware full-list path per list.
///
/// Google Tasks v1 has no `syncToken`; `showDeleted` plus pagination is the
/// documented way to observe remote deletions.
///
/// # Errors
///
/// Returns an error if the task list index cannot be fetched.
pub async fn sync(pool: &SqlitePool, access_token: &str) -> Result<()> {
    let client = Client::new();
    let lists = fetch_task_lists(&client, access_token).await?;

    for list_id in lists {
        if let Err(e) = sync_task_list(pool, &client, access_token, &list_id).await {
            eprintln!("Google Tasks sync error for list {list_id}: {e}");
        }
    }
    Ok(())
}

async fn fetch_task_lists(client: &Client, access_token: &str) -> Result<Vec<String>> {
    let mut lists = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut url = url::Url::parse(&format!("{TASKS_BASE}/users/@me/lists"))?;
        if let Some(token) = &page_token {
            url.query_pairs_mut().append_pair("pageToken", token);
        }

        let response = client
            .get(url.as_str())
            .bearer_auth(access_token)
            .send()
            .await?;
        if !response.status().is_success() {
            let err = response.text().await?;
            return Err(color_eyre::eyre::eyre!(
                "Google Tasks API error (lists): {err}"
            ));
        }

        let page: GoogleTaskLists = response.json().await?;
        if let Some(items) = page.items {
            lists.extend(items.into_iter().map(|list| list.id));
        }
        match page.next_page_token {
            Some(token) => page_token = Some(token),
            None => break,
        }
    }

    Ok(lists)
}

async fn sync_task_list(
    pool: &SqlitePool,
    client: &Client,
    access_token: &str,
    list_id: &str,
) -> Result<()> {
    let mut page_token: Option<String> = None;

    loop {
        let encoded = urlencoding::encode(list_id);
        let mut url = url::Url::parse(&format!("{TASKS_BASE}/lists/{encoded}/tasks"))?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("showCompleted", "true");
            query.append_pair("showHidden", "true");
            // Deleted tasks must be requested explicitly, otherwise a remote
            // deletion is invisible and the local copy lingers forever.
            query.append_pair("showDeleted", "true");
            if let Some(token) = &page_token {
                query.append_pair("pageToken", token);
            }
        }

        let response = client
            .get(url.as_str())
            .bearer_auth(access_token)
            .send()
            .await?;
        if !response.status().is_success() {
            let err = response.text().await?;
            return Err(color_eyre::eyre::eyre!(
                "Google Tasks API error (list {list_id}): {err}"
            ));
        }

        let mut task_list: GoogleTaskList = response.json().await?;
        if let Some(tasks) = task_list.items.take() {
            for task in tasks {
                if let Err(e) = apply_task(pool, list_id, task).await {
                    eprintln!("Failed to upsert Google task: {e}");
                }
            }
        }

        match task_list.next_page_token {
            Some(token) => page_token = Some(token),
            None => break,
        }
    }

    Ok(())
}

async fn apply_task(
    pool: &SqlitePool,
    list_id: &str,
    task: GoogleTask,
) -> Result<(), sqlx::Error> {
    let app_task_id = format!("google_{}", task.id);

    if let Some(local_task) = crate::db::get_task(pool, &app_task_id).await? {
        // Offline-first: a locally edited row outranks the remote copy.
        if local_task.dirty == Some(true) {
            return Ok(());
        }
        let remote_updated = task
            .updated
            .clone()
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
        if local_task
            .updated_at
            .as_deref()
            .is_some_and(|updated| updated > remote_updated.as_str())
        {
            return Ok(());
        }
    }

    if task.deleted.unwrap_or(false) {
        return crate::db::delete_task(pool, app_task_id).await;
    }

    let remote_updated_at = task
        .updated
        .clone()
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

    let status = match task.status.as_deref() {
        Some("completed") => Some(crate::domain::AppTaskStatus::Done),
        _ => Some(crate::domain::AppTaskStatus::Todo),
    };

    let app_task = crate::domain::AppTask {
        id: crate::domain::TaskId(app_task_id),
        title: task.title.unwrap_or_else(|| "No Title".to_string()),
        status,
        priority: None,
        tags: None,
        checklist: None,
        parent_task: task.parent.map(crate::domain::TaskId),
        dependencies: None,
        est: None,
        added: Some(chrono::Utc::now().to_rfc3339()),
        canvas_x: None,
        canvas_y: None,
        on_canvas: None,
        remote_id: Some(crate::domain::RemoteId(task.id)),
        notes: task.notes,
        tabs: None,
        due: task.due,
        updated_at: Some(remote_updated_at),
        etag: None,
        dirty: Some(false),
        task_list_id: Some(list_id.to_string()),
    };

    crate::db::upsert_task(pool, app_task).await
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

    let list = task.task_list_id.as_deref().unwrap_or("@default");
    let list = urlencoding::encode(list);

    let (url, method) = task.remote_id.as_ref().map_or_else(
        || {
            (
                format!("{TASKS_BASE}/lists/{list}/tasks"),
                reqwest::Method::POST,
            )
        },
        |remote_id| {
            (
                format!("{TASKS_BASE}/lists/{list}/tasks/{remote_id}"),
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
        return Err(color_eyre::eyre::eyre!(
            "Failed to publish Google Task: {err}"
        ));
    }

    let created: GoogleTask = response.json().await?;
    Ok(created.id)
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn delete(remote_id: &str, access_token: &str) -> Result<()> {
    let client = Client::new();
    let url = format!("{TASKS_BASE}/lists/@default/tasks/{remote_id}");
    let response = client
        .request(reqwest::Method::DELETE, &url)
        .bearer_auth(access_token)
        .send()
        .await?;

    if !response.status().is_success() {
        let err = response.text().await?;
        return Err(color_eyre::eyre::eyre!(
            "Failed to delete Google Task: {err}"
        ));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn google_task(id: &str, deleted: bool) -> GoogleTask {
        GoogleTask {
            id: id.into(),
            title: Some("Task".into()),
            notes: None,
            due: None,
            status: Some("needsAction".into()),
            parent: None,
            updated: Some("2026-09-02T10:00:00Z".into()),
            deleted: Some(deleted),
        }
    }

    fn local_task(id: &str, remote_id: &str, list_id: &str, dirty: bool) -> crate::domain::AppTask {
        crate::domain::AppTask {
            id: crate::domain::TaskId(id.into()),
            title: "Local".into(),
            status: Some(crate::domain::AppTaskStatus::Todo),
            priority: None,
            tags: None,
            checklist: None,
            parent_task: None,
            dependencies: None,
            est: None,
            added: None,
            canvas_x: None,
            canvas_y: None,
            on_canvas: None,
            remote_id: Some(crate::domain::RemoteId(remote_id.into())),
            notes: None,
            tabs: None,
            due: None,
            updated_at: Some("2026-09-01T00:00:00Z".into()),
            etag: None,
            dirty: Some(dirty),
            task_list_id: Some(list_id.into()),
        }
    }

    #[tokio::test]
    async fn remotely_deleted_task_is_removed_locally() {
        let pool = crate::db::init_db("sqlite::memory:").await.unwrap();
        crate::db::create_task(&pool, local_task("google_t1", "t1", "list-a", false))
            .await
            .unwrap();

        apply_task(&pool, "list-a", google_task("t1", true))
            .await
            .unwrap();

        assert!(crate::db::get_task(&pool, "google_t1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn remotely_deleted_task_survives_when_locally_dirty() {
        let pool = crate::db::init_db("sqlite::memory:").await.unwrap();
        crate::db::create_task(&pool, local_task("google_t1", "t1", "list-a", true))
            .await
            .unwrap();

        apply_task(&pool, "list-a", google_task("t1", true))
            .await
            .unwrap();

        assert!(crate::db::get_task(&pool, "google_t1").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn synced_task_records_its_list() {
        let pool = crate::db::init_db("sqlite::memory:").await.unwrap();

        apply_task(&pool, "list-work", google_task("t2", false))
            .await
            .unwrap();

        let stored = crate::db::get_task(&pool, "google_t2")
            .await
            .unwrap()
            .expect("task should be stored");
        assert_eq!(stored.task_list_id.as_deref(), Some("list-work"));
    }
}
