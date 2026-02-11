use rusqlite::{params, Connection};

use crate::model::SearchItem;

pub fn open_memory() -> Result<Connection, rusqlite::Error> {
    let conn = Connection::open_in_memory()?;
    conn.execute(
        "CREATE TABLE item (id TEXT PRIMARY KEY, kind TEXT, title TEXT, path TEXT)",
        [],
    )?;
    Ok(conn)
}

pub fn upsert_item(db: &Connection, item: &SearchItem) -> Result<(), rusqlite::Error> {
    db.execute(
        "INSERT INTO item (id, kind, title, path) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET kind=excluded.kind, title=excluded.title, path=excluded.path",
        params![item.id, item.kind, item.title, item.path],
    )?;
    Ok(())
}

pub fn get_item(db: &Connection, id: &str) -> Result<Option<SearchItem>, rusqlite::Error> {
    let mut stmt = db.prepare("SELECT id, kind, title, path FROM item WHERE id = ?1")?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(SearchItem {
            id: row.get(0)?,
            kind: row.get(1)?,
            title: row.get(2)?,
            path: row.get(3)?,
        }))
    } else {
        Ok(None)
    }
}
