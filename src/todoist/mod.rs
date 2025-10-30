use anyhow::{Result, anyhow};
use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use serde::Serialize;
use std::result::Result as StdResult;

use crate::todoist::http::{
    TodoistHttpClient,
    models::{Project, Section, Task},
};

pub mod http;

pub async fn get_tasks_due_today<Z>(client: &TodoistHttpClient, timezone: Z) -> Result<Vec<Task>>
where
    Z: TimeZone,
{
    let all_tasks = client.get_all::<Task>("/tasks").await?;
    let mut today_tasks = vec![];

    let today = Utc::now().with_timezone(&timezone).fixed_offset();
    let today_date = today.date_naive();

    for task in all_tasks {
        if let Some(due) = &task.due {
            let due_date: StdResult<DateTime<FixedOffset>, _> = due.clone().try_into();
            if let Ok(due_date) = due_date {
                let due_date_in_tz = due_date.with_timezone(&timezone);
                if due_date_in_tz.date_naive() == today_date {
                    today_tasks.push(task);
                }
            } else {
                continue;
            }
        }
    }

    // Sort tasks by their due time
    today_tasks.sort_by_key(|task| {
        if let Some(due) = &task.due {
            let due_date: StdResult<DateTime<FixedOffset>, _> = due.clone().try_into();
            if let Ok(due_date) = due_date {
                return due_date.time();
            }
        }
        chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap()
    });

    Ok(today_tasks)
}

pub async fn get_projects(client: &TodoistHttpClient) -> Result<Vec<Project>> {
    client
        .get_all::<Project>("/projects")
        .await
        .map_err(|e| anyhow!(e))
}

pub async fn get_sections(client: &TodoistHttpClient, project_id: &str) -> Result<Vec<Section>> {
    client
        .get_all::<Section>(&format!("/sections?project_id={}", project_id))
        .await
        .map_err(|e| anyhow!(e))
}

#[derive(Serialize, Debug, Default)]
pub struct NewTask {
    pub content: String,
    pub description: Option<String>,
    pub project_id: Option<String>,
    pub section_id: Option<String>,
    pub parent_id: Option<String>,
    pub order: Option<u32>,
    pub labels: Option<Vec<String>>,
    pub priority: Option<u8>,
    pub assignee_id: Option<String>,
    pub due_string: Option<String>,
    pub due_date: Option<String>,
    pub due_datetime: Option<String>,
    pub due_lang: Option<String>,
    pub duration: Option<u32>,
    pub duration_unit: Option<String>,
    pub deadline_date: Option<String>,
}

pub async fn create_task(client: &TodoistHttpClient, new_task: NewTask) -> Result<Task> {
    client
        .post("/tasks")
        .json(&new_task)
        .send()
        .await?
        .json()
        .await
        .map_err(|e| anyhow!(e))
}
