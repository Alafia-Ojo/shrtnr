use rusqlite::{Connection, OptionalExtension, Result, params};

pub type LinkInfo = (String, i64, Option<String>, bool);
pub type LinkRow = (String, String, i64, String, Option<String>, bool);

pub fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS links (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            short_code  TEXT NOT NULL UNIQUE,
            original_url TEXT NOT NULL,
            visits      INTEGER NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE IF NOT EXISTS click_events (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            short_code  TEXT NOT NULL,
            referrer    TEXT NOT NULL DEFAULT '',
            user_agent  TEXT NOT NULL DEFAULT '',
            ip          TEXT NOT NULL DEFAULT '',
            clicked_at  TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (short_code) REFERENCES links(short_code)
        );
        CREATE INDEX IF NOT EXISTS idx_short_code ON links(short_code);
        CREATE INDEX IF NOT EXISTS idx_click_short_code ON click_events(short_code);",
    )?;
    // migrate: add creator_id column for per-user link isolation
    let _ = conn.execute(
        "ALTER TABLE links ADD COLUMN creator_id TEXT NOT NULL DEFAULT ''",
        [],
    );
    let _ = conn.execute("ALTER TABLE links ADD COLUMN expires_at TEXT", []);
    Ok(())
}

pub fn insert_link(
    conn: &Connection,
    short_code: &str,
    original_url: &str,
    creator_id: &str,
    expires_hours: Option<i64>,
) -> Result<()> {
    match expires_hours {
        Some(h) => {
            let modifier = format!("+{} hours", h);
            conn.execute(
                "INSERT INTO links (short_code, original_url, creator_id, expires_at) VALUES (?1, ?2, ?3, datetime('now', ?4))",
                params![short_code, original_url, creator_id, modifier],
            )?;
        }
        None => {
            conn.execute(
                "INSERT INTO links (short_code, original_url, creator_id) VALUES (?1, ?2, ?3)",
                params![short_code, original_url, creator_id],
            )?;
        }
    }
    Ok(())
}

pub fn get_link(conn: &Connection, short_code: &str) -> Result<Option<LinkInfo>> {
    conn.query_row(
        "SELECT original_url, visits, expires_at, (expires_at IS NOT NULL AND expires_at <= datetime('now')) FROM links WHERE short_code = ?1",
        params![short_code],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get::<_, i64>(3)? != 0)),
    )
    .optional()
}

pub fn increment_visits(conn: &Connection, short_code: &str) -> Result<()> {
    conn.execute(
        "UPDATE links SET visits = visits + 1 WHERE short_code = ?1",
        params![short_code],
    )?;
    Ok(())
}

pub fn log_click(
    conn: &Connection,
    short_code: &str,
    referrer: &str,
    user_agent: &str,
    ip: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO click_events (short_code, referrer, user_agent, ip) VALUES (?1, ?2, ?3, ?4)",
        params![short_code, referrer, user_agent, ip],
    )?;
    Ok(())
}

pub fn get_clicks(
    conn: &Connection,
    short_code: &str,
) -> Result<Vec<(String, String, String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT referrer, user_agent, ip, clicked_at FROM click_events WHERE short_code = ?1 ORDER BY clicked_at DESC LIMIT 50",
    )?;
    let rows = stmt.query_map(params![short_code], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    })?;
    let mut clicks = Vec::new();
    for row in rows {
        clicks.push(row?);
    }
    Ok(clicks)
}

pub fn delete_link(conn: &Connection, short_code: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM click_events WHERE short_code = ?1",
        params![short_code],
    )?;
    conn.execute(
        "DELETE FROM links WHERE short_code = ?1",
        params![short_code],
    )?;
    Ok(())
}

pub fn get_link_creator(conn: &Connection, short_code: &str) -> Result<Option<String>> {
    conn.query_row(
        "SELECT creator_id FROM links WHERE short_code = ?1",
        params![short_code],
        |row| row.get(0),
    )
    .optional()
}

pub fn get_all_links(conn: &Connection, creator_id: Option<&str>) -> Result<Vec<LinkRow>> {
    let sql = "SELECT short_code, original_url, visits, created_at, expires_at, (expires_at IS NOT NULL AND expires_at <= datetime('now')) FROM links";
    match creator_id {
        Some(id) if !id.is_empty() => {
            let mut stmt = conn.prepare(&format!(
                "{} WHERE creator_id = ?1 ORDER BY created_at DESC",
                sql
            ))?;
            let rows = stmt.query_map(params![id], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get::<_, i64>(5)? != 0,
                ))
            })?;
            let mut links = Vec::new();
            for row in rows {
                links.push(row?);
            }
            Ok(links)
        }
        _ => {
            let mut stmt = conn.prepare(&format!("{} ORDER BY created_at DESC", sql))?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get::<_, i64>(5)? != 0,
                ))
            })?;
            let mut links = Vec::new();
            for row in rows {
                links.push(row?);
            }
            Ok(links)
        }
    }
}
