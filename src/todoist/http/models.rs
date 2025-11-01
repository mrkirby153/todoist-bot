#![allow(dead_code, reason = "Models for Todoist HTTP API responses")]
use chrono::{DateTime, FixedOffset, Local, NaiveDate, NaiveDateTime, Offset};
use chrono_tz::Tz;
use serde::Deserialize;
use thiserror::Error;

use crate::get_timezone_override;

#[derive(Debug, Deserialize)]
pub struct CursorResponse<T> {
    pub results: Vec<T>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Project {
    pub id: String,
    pub can_assign_tasks: bool,
    pub child_order: i64,
    pub color: String,
    pub creator_uid: Option<String>,
    pub created_at: String,
    pub is_archived: bool,
    pub is_deleted: bool,
    pub is_favorite: bool,
    pub is_frozen: bool,
    pub name: String,
    pub updated_at: Option<String>,
    pub view_style: String,
    pub default_order: i64,
    pub description: String,
    pub public_key: String,
    pub role: Option<String>,
    pub parent_id: Option<String>,
    pub inbox_project: bool,
    pub is_collapsed: bool,
    pub is_shared: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Due {
    pub date: String,
    pub timezone: Option<String>,
    pub string: String,
    pub lang: String,
    pub is_recurring: bool,
}

#[derive(Debug, Error)]
pub enum DueParseError {
    #[error("Failed to parse date")]
    InvalidFormat,
}

impl Due {
    pub fn is_date_only(&self) -> bool {
        // If the date string is in the format YYYY-MM-DD, it's date only
        NaiveDate::parse_from_str(self.date.as_str(), "%Y-%m-%d").is_ok()
    }
}

impl TryFrom<Due> for DateTime<FixedOffset> {
    type Error = DueParseError;
    fn try_from(due: Due) -> Result<Self, DueParseError> {
        if let Ok(dt) = DateTime::parse_from_rfc3339(due.date.as_str()) {
            return Ok(dt);
        }

        // Try to parse it as a date only
        if let Ok(naive_date) = NaiveDate::parse_from_str(due.date.as_str(), "%Y-%m-%d") {
            if let Some(tz_str) = &due.timezone {
                // If timezone is provided, use it
                let tz = tz_str
                    .parse::<Tz>()
                    .map_err(|_| DueParseError::InvalidFormat)?;
                let naive_dt = naive_date
                    .and_hms_opt(0, 0, 0)
                    .ok_or(DueParseError::InvalidFormat)?;
                return Ok(naive_dt
                    .and_local_timezone(tz)
                    .single()
                    .ok_or(DueParseError::InvalidFormat)?
                    .fixed_offset());
            } else {
                let naive = naive_date
                    .and_hms_opt(0, 0, 0)
                    .ok_or(DueParseError::InvalidFormat)?;
                let naive = match get_timezone_override() {
                    Some(tz) => naive
                        .and_local_timezone(tz)
                        .single()
                        .ok_or(DueParseError::InvalidFormat)?
                        .fixed_offset(),
                    None => naive
                        .and_local_timezone(Local)
                        .single()
                        .ok_or(DueParseError::InvalidFormat)?
                        .fixed_offset(),
                };
                return Ok(naive);
            }
        }

        // Try to parse it as a datetime without timezone
        if let Ok(dt) = NaiveDateTime::parse_from_str(due.date.as_str(), "%Y-%m-%dT%H:%M:%S") {
            // Use the local timezone if none is provided
            let local_tz = chrono::Local::now().offset().fix();
            let dt = dt
                .and_local_timezone(local_tz)
                .single()
                .ok_or(DueParseError::InvalidFormat)?;
            return Ok(dt);
        }

        Err(DueParseError::InvalidFormat)
    }
}

#[derive(Debug, Deserialize)]
pub struct Deadline {
    pub date: String,
    pub lang: String,
}

#[derive(Debug, Deserialize)]
pub struct Duration {
    pub amount: i64,
    pub unit: String,
}

#[derive(Debug, Deserialize)]
pub struct Task {
    pub user_id: String,
    pub id: String,
    pub project_id: String,
    pub section_id: Option<String>,
    pub parent_id: Option<String>,
    pub added_by_uid: Option<String>,
    pub assigned_by_uid: Option<String>,
    pub responsible_uid: Option<String>,
    pub labels: Vec<String>,
    pub deadline: Option<Deadline>,
    pub duration: Option<Duration>,
    pub checked: bool,
    pub is_deleted: bool,
    pub added_at: String,
    pub completed_at: Option<String>,
    pub updated_at: Option<String>,
    pub due: Option<Due>,
    pub priority: i64,
    pub child_order: i64,
    pub content: String,
    pub description: String,
    pub day_order: i64,
    pub is_collapsed: bool,
}

impl Task {
    pub fn get_url(&self) -> String {
        format!("https://app.todoist.com/app/task/{}", self.id)
    }
}

#[derive(Deserialize, Debug)]
pub struct Section {
    pub id: String,
    pub user_id: String,
    pub project_id: String,
    pub added_at: String,
    pub updated_at: Option<String>,
    pub archived_at: Option<String>,
    pub name: String,
    pub section_order: i64,
    pub is_archived: bool,
    pub is_deleted: bool,
    pub is_collapsed: bool,
}
