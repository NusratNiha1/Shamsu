use anyhow::Result;
use chrono::{DateTime, Utc};
use dirs::home_dir;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// Returns the path to the Shamsu data directory (~/.shamsu)
pub fn shamsu_dir() -> PathBuf {
    home_dir()
        .expect("Cannot find home directory")
        .join(".shamsu")
}

/// Returns the path to the SQLite database
pub fn db_path() -> PathBuf {
    shamsu_dir().join("shamsu.db")
}

/// Creates all required directories and initializes the database schema
pub async fn init() -> Result<()> {
    let dir = shamsu_dir();
    std::fs::create_dir_all(&dir)?;
    std::fs::create_dir_all(dir.join("skills"))?;
    std::fs::create_dir_all(dir.join("sessions"))?;
    std::fs::create_dir_all(dir.join("models"))?;

    let conn = open()?;
    create_schema(&conn)?;
    Ok(())
}

/// Opens a connection to the SQLite database
pub fn open() -> Result<Connection> {
    let conn = Connection::open(db_path())?;
    // Enable WAL mode for better concurrent access
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    Ok(conn)
}

/// Creates all database tables if they don't exist
fn create_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS sessions (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL UNIQUE,
            workspace   TEXT NOT NULL,
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL,
            is_active   INTEGER NOT NULL DEFAULT 0,
            permission_profile TEXT NOT NULL DEFAULT 'standard',
            active_skills TEXT NOT NULL DEFAULT '[]'
        );

        CREATE TABLE IF NOT EXISTS messages (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id  TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
            role        TEXT NOT NULL CHECK(role IN ('user','assistant','system','tool')),
            content     TEXT NOT NULL,
            created_at  TEXT NOT NULL,
            token_count INTEGER,
            is_archived INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS memory_snapshots (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id  TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
            summary     TEXT NOT NULL,
            covers_up_to INTEGER NOT NULL,
            created_at  TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS config (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);
        CREATE INDEX IF NOT EXISTS idx_messages_archived ON messages(session_id, is_archived);
        ",
    )?;
    Ok(())
}

// ─── Session model ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub workspace: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
    pub permission_profile: String,
    pub active_skills: Vec<String>,
}

impl Session {
    pub fn new(name: &str, workspace: &str) -> Self {
        let now = Utc::now();
        Session {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            workspace: workspace.to_string(),
            created_at: now,
            updated_at: now,
            is_active: false,
            permission_profile: "standard".to_string(),
            active_skills: vec![],
        }
    }
}

// ─── Session CRUD ─────────────────────────────────────────────────────────────

pub fn create_session(session: &Session) -> Result<()> {
    let conn = open()?;
    let skills_json = serde_json::to_string(&session.active_skills)?;
    conn.execute(
        "INSERT INTO sessions (id, name, workspace, created_at, updated_at, is_active, permission_profile, active_skills)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            session.id,
            session.name,
            session.workspace,
            session.created_at.to_rfc3339(),
            session.updated_at.to_rfc3339(),
            session.is_active as i32,
            session.permission_profile,
            skills_json,
        ],
    )?;
    Ok(())
}

pub fn get_session_by_name(name: &str) -> Result<Option<Session>> {
    let conn = open()?;
    let mut stmt = conn.prepare(
        "SELECT id, name, workspace, created_at, updated_at, is_active, permission_profile, active_skills
         FROM sessions WHERE name = ?1",
    )?;
    let mut rows = stmt.query_map(params![name], map_session_row)?;
    Ok(rows.next().transpose()?)
}

#[allow(dead_code)]
pub fn get_session_by_id(id: &str) -> Result<Option<Session>> {
    let conn = open()?;
    let mut stmt = conn.prepare(
        "SELECT id, name, workspace, created_at, updated_at, is_active, permission_profile, active_skills
         FROM sessions WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(params![id], map_session_row)?;
    Ok(rows.next().transpose()?)
}

pub fn list_sessions() -> Result<Vec<Session>> {
    let conn = open()?;
    let mut stmt = conn.prepare(
        "SELECT id, name, workspace, created_at, updated_at, is_active, permission_profile, active_skills
         FROM sessions ORDER BY updated_at DESC",
    )?;
    let rows = stmt.query_map([], map_session_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn get_active_session() -> Result<Option<Session>> {
    let conn = open()?;
    let mut stmt = conn.prepare(
        "SELECT id, name, workspace, created_at, updated_at, is_active, permission_profile, active_skills
         FROM sessions WHERE is_active = 1 LIMIT 1",
    )?;
    let mut rows = stmt.query_map([], map_session_row)?;
    Ok(rows.next().transpose()?)
}

pub fn set_active_session(id: &str) -> Result<()> {
    let conn = open()?;
    conn.execute("UPDATE sessions SET is_active = 0", [])?;
    conn.execute(
        "UPDATE sessions SET is_active = 1, updated_at = ?1 WHERE id = ?2",
        params![Utc::now().to_rfc3339(), id],
    )?;
    Ok(())
}

pub fn delete_session(id: &str) -> Result<()> {
    let conn = open()?;
    conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn rename_session(id: &str, new_name: &str) -> Result<()> {
    let conn = open()?;
    conn.execute(
        "UPDATE sessions SET name = ?1, updated_at = ?2 WHERE id = ?3",
        params![new_name, Utc::now().to_rfc3339(), id],
    )?;
    Ok(())
}

fn map_session_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<Session> {
    let skills_json: String = row.get(7)?;
    let active_skills: Vec<String> = serde_json::from_str(&skills_json).unwrap_or_default();
    Ok(Session {
        id: row.get(0)?,
        name: row.get(1)?,
        workspace: row.get(2)?,
        created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
            .unwrap()
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
            .unwrap()
            .with_timezone(&Utc),
        is_active: row.get::<_, i32>(5)? != 0,
        permission_profile: row.get(6)?,
        active_skills,
    })
}

// ─── Messages ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Option<i64>,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub token_count: Option<i32>,
    pub is_archived: bool,
}

impl Message {
    pub fn new(session_id: &str, role: &str, content: &str) -> Self {
        Message {
            id: None,
            session_id: session_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            created_at: Utc::now(),
            token_count: None,
            is_archived: false,
        }
    }
}

pub fn append_message(msg: &Message) -> Result<i64> {
    let conn = open()?;
    conn.execute(
        "INSERT INTO messages (session_id, role, content, created_at, token_count, is_archived)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            msg.session_id,
            msg.role,
            msg.content,
            msg.created_at.to_rfc3339(),
            msg.token_count,
            msg.is_archived as i32,
        ],
    )?;
    let id = conn.last_insert_rowid();
    // Update session's updated_at
    conn.execute(
        "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
        params![Utc::now().to_rfc3339(), msg.session_id],
    )?;
    Ok(id)
}

pub fn get_messages(session_id: &str, limit: Option<usize>) -> Result<Vec<Message>> {
    let conn = open()?;
    let sql = match limit {
        Some(n) => format!(
            "SELECT id, session_id, role, content, created_at, token_count, is_archived
             FROM messages WHERE session_id = '{session_id}' AND is_archived = 0
             ORDER BY id DESC LIMIT {n}"
        ),
        None => format!(
            "SELECT id, session_id, role, content, created_at, token_count, is_archived
             FROM messages WHERE session_id = '{session_id}' AND is_archived = 0
             ORDER BY id ASC"
        ),
    };

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| {
        Ok(Message {
            id: Some(row.get(0)?),
            session_id: row.get(1)?,
            role: row.get(2)?,
            content: row.get(3)?,
            created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                .unwrap()
                .with_timezone(&Utc),
            token_count: row.get(5)?,
            is_archived: row.get::<_, i32>(6)? != 0,
        })
    })?;

    let mut messages: Vec<Message> = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    if limit.is_some() {
        messages.reverse(); // return in chronological order
    }
    Ok(messages)
}

pub fn archive_messages_before(session_id: &str, before_id: i64) -> Result<usize> {
    let conn = open()?;
    let n = conn.execute(
        "UPDATE messages SET is_archived = 1 WHERE session_id = ?1 AND id <= ?2",
        params![session_id, before_id],
    )?;
    Ok(n)
}

// ─── Memory snapshots ─────────────────────────────────────────────────────────

pub fn save_snapshot(session_id: &str, summary: &str, covers_up_to: i64) -> Result<()> {
    let conn = open()?;
    conn.execute(
        "INSERT INTO memory_snapshots (session_id, summary, covers_up_to, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![session_id, summary, covers_up_to, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

pub fn get_latest_snapshot(session_id: &str) -> Result<Option<String>> {
    let conn = open()?;
    let mut stmt = conn.prepare(
        "SELECT summary FROM memory_snapshots WHERE session_id = ?1 ORDER BY id DESC LIMIT 1",
    )?;
    let mut rows = stmt.query_map(params![session_id], |row| row.get::<_, String>(0))?;
    Ok(rows.next().transpose()?)
}

// ─── Config ───────────────────────────────────────────────────────────────────

pub fn get_config(key: &str) -> Result<Option<String>> {
    let conn = open()?;
    let mut stmt = conn.prepare("SELECT value FROM config WHERE key = ?1")?;
    let mut rows = stmt.query_map(params![key], |row| row.get::<_, String>(0))?;
    Ok(rows.next().transpose()?)
}

pub fn set_config(key: &str, value: &str) -> Result<()> {
    let conn = open()?;
    conn.execute(
        "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
        params![key, value],
    )?;
    Ok(())
}

pub fn list_config() -> Result<Vec<(String, String)>> {
    let conn = open()?;
    let mut stmt = conn.prepare("SELECT key, value FROM config ORDER BY key")?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}
