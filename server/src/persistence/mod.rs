//! DB operations and schema migrations
//!
//! This code uses several [`Diesel ORM`](http://diesel.rs/) tools for DB operations:
//! - [`diesel-migrations`](https://docs.rs/diesel_migrations/1.4.0/diesel_migrations/)
//!   for managing table migrations
//! - [`diesel-cli`](https://github.com/diesel-rs/diesel/tree/master/diesel_cli/)
//!   for generating and testing migrations

pub mod character;

mod conversions;
mod error;
mod json_models;
mod models;
mod schema;

extern crate diesel;

use diesel::{connection::SimpleConnection, prelude::*};
use diesel_migrations::embed_migrations;
use std::{env, fs, path::PathBuf};
use tracing::warn;

// See: https://docs.rs/diesel_migrations/1.4.0/diesel_migrations/macro.embed_migrations.html
// This macro is called at build-time, and produces the necessary migration info
// for the `embedded_migrations` call below.
//
// NOTE: Adding a useless comment to trigger the migrations being run.  Delete
// when needed.
embed_migrations!();

/// Runs any pending database migrations. This is executed during server startup
pub fn run_migrations(db_dir: &str) -> Result<(), diesel_migrations::RunMigrationsError> {
    let db_dir = &apply_saves_dir_override(db_dir);
    let _ = fs::create_dir(format!("{}/", db_dir));
    embedded_migrations::run_with_output(
        &establish_connection(db_dir).expect(
            "If we cannot execute migrations, we should not be allowed to launch the server, so \
             we don't populate it with bad data.",
        ),
        &mut std::io::stdout(),
    )
}

fn establish_connection(db_dir: &str) -> QueryResult<SqliteConnection> {
    let db_dir = &apply_saves_dir_override(db_dir);
    let database_url = format!("{}/db.sqlite", db_dir);

    let connection = SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));

    // Use Write-Ahead-Logging for improved concurrency: https://sqlite.org/wal.html
    // Set a busy timeout (in ms): https://sqlite.org/c3ref/busy_timeout.html
    connection
        .batch_execute(
            "
        PRAGMA foreign_keys = ON;
        PRAGMA journal_mode = WAL;
        PRAGMA busy_timeout = 250;
        ",
        )
        .expect(
            "Failed adding PRAGMA statements while establishing sqlite connection, including \
             enabling foreign key constraints.  We will not allow connecting to the server under \
             these conditions.",
        );

    Ok(connection)
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
