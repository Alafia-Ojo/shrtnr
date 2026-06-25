use rusqlite::{Connection, OptionalExtension, Result, params};

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
    Ok(())
}

pub fn insert_link(conn: &Connection, short_code: &str, original_url: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO links (short_code, original_url) VALUES (?1, ?2)",
        params![short_code, original_url],
    )?;
    Ok(())
}

pub fn get_link(conn: &Connection, short_code: &str) -> Result<Option<(String, i64)>> {
    conn.query_row(
        "SELECT original_url, visits FROM links WHERE short_code = ?1",
        params![short_code],
        |row| Ok((row.get(0)?, row.get(1)?)),
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

pub fn get_all_links(conn: &Connection) -> Result<Vec<(String, String, i64, String)>> {
    let mut stmt = conn.prepare(
        "SELECT short_code, original_url, visits, created_at FROM links ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    })?;
    let mut links = Vec::new();
    for row in rows {
        links.push(row?);
    }
    Ok(links)
}
