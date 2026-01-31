use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 记忆条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: i64,
    pub kind: String,
    pub content: String,
    pub emotional_weight: f32,
    pub timestamp: u64,
}

/// SQLite 记忆系统
pub struct MemoryStore {
    conn: Connection,
}

impl MemoryStore {
    /// 打开或创建数据库
    pub fn open(db_path: &Path) -> Result<Self, String> {
        let conn = Connection::open(db_path)
            .map_err(|e| format!("Failed to open DB: {}", e))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS memories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                kind TEXT NOT NULL,
                content TEXT NOT NULL,
                emotional_weight REAL NOT NULL DEFAULT 0.5,
                timestamp INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_memories_ts ON memories(timestamp DESC);
            CREATE INDEX IF NOT EXISTS idx_memories_kind ON memories(kind);

            CREATE TABLE IF NOT EXISTS sophie_state (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );"
        ).map_err(|e| format!("Failed to init DB: {}", e))?;

        Ok(Self { conn })
    }

    /// 添加记忆
    pub fn add(&self, kind: &str, content: &str, emotional_weight: f32) -> Result<i64, String> {
        let now = unix_now();
        self.conn.execute(
            "INSERT INTO memories (kind, content, emotional_weight, timestamp) VALUES (?1, ?2, ?3, ?4)",
            params![kind, content, emotional_weight, now],
        ).map_err(|e| format!("Insert error: {}", e))?;
        Ok(self.conn.last_insert_rowid())
    }

    /// 获取最近 N 条记忆
    pub fn recent(&self, count: usize) -> Vec<Memory> {
        let mut stmt = self.conn
            .prepare("SELECT id, kind, content, emotional_weight, timestamp FROM memories ORDER BY timestamp DESC LIMIT ?1")
            .unwrap();
        stmt.query_map(params![count as i64], |row| {
            Ok(Memory {
                id: row.get(0)?,
                kind: row.get(1)?,
                content: row.get(2)?,
                emotional_weight: row.get(3)?,
                timestamp: row.get(4)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
    }

    /// 获取最近 N 条记忆的摘要文本（用于 LLM prompt）
    pub fn recent_as_text(&self, count: usize) -> Vec<String> {
        self.recent(count)
            .iter()
            .map(|m| format!("[{}] {}", m.kind, m.content))
            .collect()
    }

    /// 获取记忆总数
    pub fn count(&self) -> i64 {
        self.conn
            .query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))
            .unwrap_or(0)
    }

    /// 保存 Sophie 的持久化状态（JSON）
    pub fn save_state(&self, key: &str, value: &str) -> Result<(), String> {
        self.conn.execute(
            "INSERT OR REPLACE INTO sophie_state (key, value) VALUES (?1, ?2)",
            params![key, value],
        ).map_err(|e| format!("Save state error: {}", e))?;
        Ok(())
    }

    /// 读取 Sophie 的持久化状态
    pub fn load_state(&self, key: &str) -> Option<String> {
        self.conn
            .query_row(
                "SELECT value FROM sophie_state WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .ok()
    }
}

fn unix_now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
