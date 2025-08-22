//! # ODB backend implementation: SQLite
//! The following is a port of libgit2-backends' `sqlite/sqlite.c` file.

use git2::odb_backend::{OdbBackend, OdbBackendAllocation, OdbBackendContext, SupportedOperations};
use git2::{Error, ErrorClass, ErrorCode, ObjectType, Oid};
use libgit2_sys as raw;
use rusqlite::Error as RusqliteError;
use rusqlite::{params, OptionalExtension};
use std::convert::Infallible;

pub struct SqliteBackend {
    conn: rusqlite::Connection,
}

const ST_READ: &str = "SELECT type, size, data FROM 'git2_odb' WHERE oid = ?;";
const ST_READ_HEADER: &str = "SELECT type, size FROM 'git2_odb' WHERE `oid` = ?;";
const ST_WRITE: &str = "INSERT OR IGNORE INTO 'git2_odb' VALUES (?, ?, ?, ?)";

impl SqliteBackend {
    pub fn new(conn: rusqlite::Connection) -> rusqlite::Result<Self> {
        // Check if we need to create the git2_odb table
        if conn.table_exists(None, "git2_odb")? {
            // Table exists, do nothing
        } else {
            conn.execute("CREATE TABLE 'git2_odb' ('oid' CHARACTER(20) PRIMARY KET NOT NULL, 'type' INTEGER NOT NULL, 'size' INTEGER NOT NULL, 'data' BLOB)", params![])?;
        }

        conn.prepare_cached(ST_READ)?;
        conn.prepare_cached(ST_READ_HEADER)?;
        conn.prepare_cached(ST_WRITE)?;

        Ok(Self { conn })
    }
}

impl OdbBackend for SqliteBackend {
    type Writepack = Infallible;
    type ReadStream = Infallible;
    type WriteStream = Infallible;

    fn supported_operations(&self) -> SupportedOperations {
        SupportedOperations::READ
            | SupportedOperations::READ_HEADER
            | SupportedOperations::WRITE
            | SupportedOperations::EXISTS
    }

    fn read(
        &mut self,
        ctx: &OdbBackendContext,
        oid: Oid,
        object_type: &mut ObjectType,
        data: &mut OdbBackendAllocation,
    ) -> Result<(), Error> {
        let row = self
            .conn
            .prepare_cached(ST_READ)
            .map_err(map_sqlite_err)?
            .query_one(params![oid.as_bytes()], |row| {
                let object_type: raw::git_object_t = row.get(0)?;
                let size: usize = row.get(1)?;
                let data: Box<[u8]> = row.get(2)?;
                Ok((ObjectType::from_raw(object_type).unwrap(), size, data))
            })
            .map_err(map_sqlite_err)?;
        *object_type = row.0;
        *data = ctx.try_alloc(row.1)?;
        data.as_mut().copy_from_slice(&row.2);
        Ok(())
    }

    fn read_header(
        &mut self,
        _ctx: &OdbBackendContext,
        oid: Oid,
        length: &mut usize,
        object_type: &mut ObjectType,
    ) -> Result<(), Error> {
        let row = self
            .conn
            .prepare_cached(ST_READ_HEADER)
            .map_err(map_sqlite_err)?
            .query_one(params![oid.as_bytes()], |row| {
                let object_type: raw::git_object_t = row.get(0)?;
                let size: usize = row.get(1)?;
                Ok((ObjectType::from_raw(object_type).unwrap(), size))
            })
            .map_err(map_sqlite_err)?;
        *object_type = row.0;
        *length = row.1;
        Ok(())
    }

    fn write(
        &mut self,
        _ctx: &OdbBackendContext,
        oid: Oid,
        object_type: ObjectType,
        data: &[u8],
    ) -> Result<(), Error> {
        self.conn
            .prepare_cached(ST_WRITE)
            .map_err(map_sqlite_err)?
            .execute(params![
                oid.as_bytes(),
                object_type.raw(),
                oid.as_bytes().len(),
                data
            ])
            .map_err(map_sqlite_err)?;
        Ok(())
    }

    fn exists(&mut self, _ctx: &OdbBackendContext, oid: Oid) -> Result<bool, Error> {
        let row = self
            .conn
            .prepare_cached(ST_READ_HEADER)
            .map_err(map_sqlite_err)?
            .query_one(params![oid.as_bytes()], |_| Ok(()))
            .optional()
            .map_err(map_sqlite_err)?;
        Ok(row.is_some())
    }
}

fn map_sqlite_err(err: RusqliteError) -> Error {
    match err {
        RusqliteError::QueryReturnedNoRows => {
            Error::new(ErrorCode::NotFound, ErrorClass::None, "not found")
        }
        _ => Error::new(ErrorCode::GenericError, ErrorClass::Object, err.to_string()),
    }
}

fn main() {
    todo!("Demonstrate how to use SqliteBackend")
}
