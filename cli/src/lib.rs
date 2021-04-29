pub mod db;
pub mod diff;

use std::env::current_dir;
use std::error::Error;
use std::fs::{copy, create_dir, read_dir, read_to_string, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use toml::Value;

use ergol_core::{Element, Table};

use crate::diff::{diff, Diff, State};

/// Tries to sort the tables in order to avoid problems with dependencies.
pub fn order(tables: Vec<Table>) -> Vec<Table> {
    let mut current: Vec<String> = vec![];
    let mut output_tables = vec![];
    let len = tables.len();

    for _ in 0..len {
        for table in &tables {
            // Check dependencies
            if !current.contains(&table.name)
                && table.dependencies().iter().all(|x| current.contains(x))
            {
                current.push(table.name.clone());
                output_tables.push(table.clone());
            }
        }
    }

    if output_tables.len() != len {
        tables
    } else {
        output_tables
    }
}

/// Find cargo toml.
pub fn find_cargo_toml() -> Option<PathBuf> {
    let mut current = current_dir().ok()?;

    loop {
        if current.join("Cargo.toml").is_file() {
            return Some(current);
        }

        if !current.pop() {
            return None;
        }
    }
}

/// Finds the last saved db state.
pub fn last_saved_state<P: AsRef<Path>>(p: P) -> Result<(Option<u32>, State), Box<dyn Error>> {
    let p = p.as_ref();
    let mut current = 0;

    loop {
        if !p.join(format!("{}", current)).is_dir() {
            if current == 0 {
                // Last state is empty.
                return Ok((None, (vec![], vec![])));
            } else {
                return state_from_dir(p.join(format!("{}", current - 1)))
                    .map(|x| (Some(current - 1), x));
            }
        }

        current += 1;
    }
}

/// Returns the db state from a directory.
pub fn state_from_dir<P: AsRef<Path>>(path: P) -> Result<State, Box<dyn Error>> {
    let mut tables = vec![];
    let mut enums = vec![];

    for file in read_dir(path.as_ref())? {
        let path = file?.path();
        if path.extension().and_then(|x| x.to_str()) == Some("json") {
            let content = read_to_string(path)?;
            let elements: Vec<Element> = serde_json::from_str(&content)?;
            for element in elements {
                match element {
                    Element::Enum(e) => enums.push(e),
                    Element::Table(t) => tables.push(t),
                }
            }
        }
    }
    Ok((enums, order(tables)))
}

/// Tries to find the database URL in Rocket.toml or Ergol.toml.
pub fn find_db_url<P: AsRef<Path>>(path: P) -> Option<String> {
    let path = path.as_ref();

    let path = if path.join("Ergol.toml").is_file() {
        path.join("Ergol.toml")
    } else if path.join("Rocket.toml").is_file() {
        path.join("Rocket.toml")
    } else {
        return None;
    };

    let content = read_to_string(path).ok()?;
    let value = content.parse::<Value>().ok()?;

    let url = value
        .as_table()?
        .get("default")?
        .as_table()?
        .get("databases")?
        .as_table()?
        .get("database")?
        .as_table()?
        .get("url")?
        .as_str()?;

    Some(url.into())
}

/// Runs the ergol migrations.
pub async fn migrate<P: AsRef<Path>>(path: P) -> Result<(), Box<dyn Error>> {
    let path = path.as_ref();
    let db_url = find_db_url(&path).unwrap();

    let (db, connection) = tokio_postgres::connect(&db_url, tokio_postgres::NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let current = db::current_migration(&db).await?;

    let mut current = match current {
        Some(i) => i + 1,
        None => {
            db::create_current_migration(&db).await?;
            0
        }
    };

    // We need to run migrations starting with current.
    loop {
        let path = path.join(format!("migrations/{}/up.sql", current));

        if !path.is_file() {
            break;
        }

        let up = read_to_string(path)?;
        println!("{}", up);

        db.simple_query(&up as &str).await?;
        db::set_migration(current, &db).await?;

        current += 1;
    }

    Ok(())
}

/// Returns the migration diff between last save state and current state.
pub fn current_diff<P: AsRef<Path>>(path: P) -> Result<Diff, Box<dyn Error>> {
    let path = path.as_ref();

    let last = last_saved_state(path.join("migrations"))?;
    let current = state_from_dir(path.join("migrations/current"))?;

    Ok(diff(last.1, current))
}

/// Delete the whole database.
pub async fn delete<P: AsRef<Path>>(path: P) -> Result<(), Box<dyn Error>> {
    let path = path.as_ref();
    let db_url = find_db_url(&path).unwrap();

    let (db, connection) = tokio_postgres::connect(&db_url, tokio_postgres::NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    db::clear(&db).await?;

    Ok(())
}

/// Saves the current state in a new migration.
pub fn save<P: AsRef<Path>>(p: P) -> Result<(), Box<dyn Error>> {
    let p = p.as_ref();
    let (last_index, last_state) = last_saved_state(p)?;
    let current_state = state_from_dir(p.join("current"))?;
    let current_index = match last_index {
        None => 0,
        Some(i) => i + 1,
    };

    let save_dir = p.join(format!("{}", current_index));
    create_dir(&save_dir)?;
    for f in read_dir(p.join("current"))? {
        let path = f?.path();
        copy(&path, &save_dir.join(path.file_name().unwrap()))?;
    }

    let diff = diff(last_state, current_state);
    let mut file = File::create(save_dir.join("up.sql"))?;
    file.write_all(diff.hint().as_bytes())?;

    let mut file = File::create(save_dir.join("down.sql"))?;
    file.write_all(diff.hint_revert().as_bytes())?;

    Ok(())
}
