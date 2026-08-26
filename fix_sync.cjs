const fs = require('fs');
let code = fs.readFileSync('src-tauri/src/apis/google_calendar.rs', 'utf8');

code = code.replace(
    'struct GoogleEventList {\n    items: Option<Vec<GoogleEvent>>,\n}',
    'struct GoogleEventList {\n    items: Option<Vec<GoogleEvent>>,\n    #[serde(rename = \"nextPageToken\")]\n    next_page_token: Option<String>,\n}'
);

code = code.replace(
    'let now = chrono::Utc::now().to_rfc3339();',
    'let time_min = (chrono::Utc::now() - chrono::Duration::days(90)).to_rfc3339();'
);

code = code.replace(
    /let mut url = url::Url::parse.*?if let Some\(events\) = event_list\.items \{/s,
    let mut page_token: Option<String> = None;
        loop {
            let mut url = url::Url::parse(&format!("https://www.googleapis.com/calendar/v3/calendars/{cal_id}/events"))?;
            {
                let mut q = url.query_pairs_mut();
                q.append_pair("timeMin", &time_min);
                q.append_pair("maxResults", "500");
                q.append_pair("orderBy", "updated");
                if let Some(token) = &page_token {
                    q.append_pair("pageToken", token);
                }
            }

            let response = client
                .get(url.as_str())
                .bearer_auth(access_token)
                .send()
                .await?;

            if !response.status().is_success() {
                let err = response.text().await?;
                eprintln!("Google Calendar API error for calendar {}: {}", calendar.id, err);
                break;
            }

            let mut event_list: GoogleEventList = response.json().await?;

            if let Some(events) = event_list.items.take() {
);

code = code.replace(
                    if let Err(e) = crate::db::upsert_event(pool, app_event).await {
                    eprintln!("Failed to upsert Google event: {e}");
                }
            }
        }
    },
                    if let Err(e) = crate::db::upsert_event(pool, app_event).await {
                    eprintln!("Failed to upsert Google event: {e}");
                }
            }
        }

        if let Some(next_token) = event_list.next_page_token {
            page_token = Some(next_token);
        } else {
            break;
        }
    }
    }
);

fs.writeFileSync('src-tauri/src/apis/google_calendar.rs', code);
