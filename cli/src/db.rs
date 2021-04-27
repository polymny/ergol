//! This module contains all the functions to help deal with the database.

use tokio_postgres::{Client, Error};

pub async fn current_migration(db: &Client) -> Result<Option<i32>, Error> {
    // If there is a problem with the db, this will launch an error.
    db.query("SELECT 1;", &[]).await?;

    // Which means that if this fail, it's because the ergol table doesn't exist.
    Ok(db
        .query("SELECT * FROM ergol;", &[])
        .await
        .ok()
        .map(|x| x[0].get(0)))
}

pub async fn create_current_migration(db: &Client) -> Result<(), Error> {
    let table = ergol_core::Table::current_migration().create_table();
    db.query(&table as &str, &[]).await?;
    db.query("INSERT INTO ergol VALUES(-1)", &[]).await?;
    Ok(())
}
