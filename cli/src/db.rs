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

pub async fn set_migration(value: i32, db: &Client) -> Result<(), Error> {
    db.query("UPDATE ergol SET migration = $1;", &[&value])
        .await?;
    Ok(())
}

pub async fn clear(db: &Client) -> Result<(), Error> {
    // Clear tables
    db.query(
        r#"
        DO $$ DECLARE
          r RECORD;
        BEGIN
          FOR r IN (SELECT tablename FROM pg_tables WHERE schemaname = current_schema()) LOOP
            EXECUTE 'DROP TABLE IF EXISTS ' || quote_ident(r.tablename) || ' CASCADE';
          END LOOP;
        END $$;
    "#,
        &[],
    )
    .await?;

    // Clear types
    db.query(
        r#"
        DO $$ DECLARE
            r RECORD;
        BEGIN
            FOR r IN (
                SELECT      n.nspname as schema, t.typname as type
                FROM        pg_type t
                LEFT JOIN   pg_catalog.pg_namespace n ON n.oid = t.typnamespace
                WHERE       (t.typrelid = 0 OR (SELECT c.relkind = 'c' FROM pg_catalog.pg_class c WHERE c.oid = t.typrelid))
                AND         NOT EXISTS(SELECT 1 FROM pg_catalog.pg_type el WHERE el.oid = t.typelem AND el.typarray = t.oid)
                AND         n.nspname NOT IN ('pg_catalog', 'information_schema')
            ) LOOP
                EXECUTE 'DROP TYPE IF EXISTS ' || quote_ident(r.type) || ' CASCADE';
            END LOOP;
        END $$;
    "#,
        &[]
    ).await?;

    Ok(())
}
