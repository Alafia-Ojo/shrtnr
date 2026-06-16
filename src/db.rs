use rusqlite::{Connection, OptionalExtension, Result, params};

pub fn init_db(path: &str) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS links (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            short_code  TEXT NOT NULL UNIQUE,
            original_url TEXT NOT NULL,
            visits      INTEGER NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_short_code ON links(short_code);",
    )?;
    Ok(conn)
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
