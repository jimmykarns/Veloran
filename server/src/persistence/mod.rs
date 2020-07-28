//! DB operations and schema migrations
//!
//! This code uses several [`Diesel ORM`](http://diesel.rs/) tools for DB operations:
//! - [`diesel-migrations`](https://docs.rs/diesel_migrations/1.4.0/diesel_migrations/)
//!   for managing table migrations
//! - [`diesel-cli`](https://github.com/diesel-rs/diesel/tree/master/diesel_cli/)
//!   for generating and testing migrations

pub mod character;

mod error;
mod models;
mod schema;

extern crate diesel;

use diesel::{connection::SimpleConnection, prelude::*};
use diesel_migrations::embed_migrations;
use std::{env, fs, path::PathBuf};
use tracing::{info, warn};

// See: https://docs.rs/diesel_migrations/1.4.0/diesel_migrations/macro.embed_migrations.html
// This macro is called at build-time, and produces the necessary migration info
// for the `embedded_migrations` call below.
//
// NOTE: Adding a useless comment to trigger the migrations being run.  Delete
// when needed.
embed_migrations!();

struct TracingOut;

impl std::io::Write for TracingOut {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        info!("{}", String::from_utf8_lossy(buf));
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}

/// Runs any pending database migrations. This is executed during server startup
pub fn run_migrations(db_dir: &str) -> Result<(), diesel_migrations::RunMigrationsError> {
    let db_dir = &apply_saves_dir_override(db_dir);
    let _ = fs::create_dir(format!("{}/", db_dir));

    embedded_migrations::run_with_output(
        &establish_connection(db_dir),
        &mut std::io::LineWriter::new(TracingOut),
    )
}

fn establish_connection(db_dir: &str) -> SqliteConnection {
    let db_dir = &apply_saves_dir_override(db_dir);
    let database_url = format!("{}/db.sqlite", db_dir);

    let connection = SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));

    // Use Write-Ahead-Logging for improved concurrency: https://sqlite.org/wal.html
    // Set a busy timeout (in ms): https://sqlite.org/c3ref/busy_timeout.html
    if let Err(e) = connection.batch_execute(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA busy_timeout = 250; 
        ",
    ) {
        warn!(
            ?e,
            "Failed adding PRAGMA statements while establishing sqlite connection, this will \
             result in a higher likelihood of locking errors"
        );
    }

    connection
}

fn apply_saves_dir_override(db_dir: &str) -> String {
    if let Some(saves_dir) = env::var_os("VELOREN_SAVES_DIR") {
        let path = PathBuf::from(saves_dir.clone());
        if path.exists() || path.parent().map(|x| x.exists()).unwrap_or(false) {
            // Only allow paths with valid unicode characters
            if let Some(path) = path.to_str() {
                return path.to_owned();
            }
        }
        warn!(?saves_dir, "VELOREN_SAVES_DIR points to an invalid path.");
    }
    db_dir.to_string()
}
