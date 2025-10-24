use anyhow::Result;
use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use std::result::Result as StdResult;

use crate::todoist::http::{TodoistHttpClient, models::Task};

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
