#![allow(dead_code, reason = "Models for Todoist HTTP API responses")]
use serde::Deserialize;

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
