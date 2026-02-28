use crate::cookie::error::Result;
use crate::cookie::model::{Cookie, CookieCreate, CookieUpdate};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

pub struct CookieStorage {
    conn: Connection,
}

impl CookieStorage {
    pub fn new(conn: Connection) -> Result<Self> {
        let storage = Self { conn };
        storage.init()?;
        Ok(storage)
    }

    pub fn new_in_memory() -> Result<Self> {
        Self::new(Connection::open_in_memory()?)
    }

    pub fn new_from_path(path: &str) -> Result<Self> {
        Self::new(Connection::open(path)?)
    }

    fn init(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS cookies (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                value TEXT NOT NULL,
                domain TEXT NOT NULL,
                path TEXT NOT NULL,
                secure INTEGER NOT NULL DEFAULT 0,
                http_only INTEGER NOT NULL DEFAULT 0,
                same_site TEXT,
                expires TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_cookies_domain ON cookies (domain)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_cookies_domain_name ON cookies (domain, name)",
            [],
        )?;

        Ok(())
    }

    pub fn insert(&self, cookie: CookieCreate) -> Result<i64> {
        let now = Utc::now();
        self.conn.execute(
            "INSERT INTO cookies (name, value, domain, path, secure, http_only, same_site, expires, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                cookie.name,
                cookie.value,
                cookie.domain,
                cookie.path,
                cookie.secure,
                cookie.http_only,
                cookie.same_site,
                cookie.expires.map(|dt| dt.to_rfc3339()),
                now.to_rfc3339(),
                now.to_rfc3339(),
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get(&self, id: i64) -> Result<Option<Cookie>> {
        self.conn
            .query_row(
                "SELECT id, name, value, domain, path, secure, http_only, same_site, expires, created_at, updated_at
                 FROM cookies WHERE id = ?1",
                params![id],
                Self::row_to_cookie,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn get_by_domain(&self, domain: &str) -> Result<Vec<Cookie>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, value, domain, path, secure, http_only, same_site, expires, created_at, updated_at
             FROM cookies WHERE domain = ?1",
        )?;
        let cookies = stmt
            .query_map(params![domain], Self::row_to_cookie)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(cookies)
    }

    pub fn get_all(&self) -> Result<Vec<Cookie>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, value, domain, path, secure, http_only, same_site, expires, created_at, updated_at
             FROM cookies",
        )?;
        let cookies = stmt
            .query_map([], Self::row_to_cookie)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(cookies)
    }

    pub fn update(&self, id: i64, update: CookieUpdate) -> Result<()> {
        let now = Utc::now();
        
        let mut set_clauses = vec!["updated_at = ?1".to_string()];
        let mut params = vec![now.to_rfc3339()];
        
        if let Some(name) = &update.name {
            set_clauses.push("name = ?".to_string());
            params.push(name.clone());
        }
        if let Some(value) = &update.value {
            set_clauses.push("value = ?".to_string());
            params.push(value.clone());
        }
        if let Some(path) = &update.path {
            set_clauses.push("path = ?".to_string());
            params.push(path.clone());
        }
        if let Some(secure) = update.secure {
            set_clauses.push("secure = ?".to_string());
            params.push(secure.to_string());
        }
        if let Some(http_only) = update.http_only {
            set_clauses.push("http_only = ?".to_string());
            params.push(http_only.to_string());
        }
        if let Some(same_site) = &update.same_site {
            set_clauses.push("same_site = ?".to_string());
            params.push(same_site.clone());
        }
        if let Some(expires) = update.expires {
            set_clauses.push("expires = ?".to_string());
            params.push(expires.to_rfc3339());
        }

        params.push(id.to_string());

        let sql = format!(
            "UPDATE cookies SET {} WHERE id = ?{}",
            set_clauses.join(", "),
            set_clauses.len() + 1
        );

        self.conn.execute(&sql, rusqlite::params_from_iter(params))?;
        Ok(())
    }

    pub fn delete(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM cookies WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn delete_by_domain(&self, domain: &str) -> Result<()> {
        self.conn.execute("DELETE FROM cookies WHERE domain = ?1", params![domain])?;
        Ok(())
    }

    pub fn delete_all(&self) -> Result<()> {
        self.conn.execute("DELETE FROM cookies", [])?;
        Ok(())
    }

    fn row_to_cookie(row: &rusqlite::Row) -> rusqlite::Result<Cookie> {
        let expires_str: Option<String> = row.get(8)?;
        let expires = expires_str
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let created_at_str: String = row.get(9)?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(9, rusqlite::types::Type::Text, Box::new(e)))?;

        let updated_at_str: String = row.get(10)?;
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(10, rusqlite::types::Type::Text, Box::new(e)))?;

        Ok(Cookie {
            id: row.get(0)?,
            name: row.get(1)?,
            value: row.get(2)?,
            domain: row.get(3)?,
            path: row.get(4)?,
            secure: row.get(5)?,
            http_only: row.get(6)?,
            same_site: row.get(7)?,
            expires,
            created_at,
            updated_at,
        })
    }
}
