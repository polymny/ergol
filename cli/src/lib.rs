pub mod db;

use std::env::current_dir;
use std::error::Error;
use std::fs::{copy, create_dir, read_dir, read_to_string, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use toml::Value;

use ergol_core::{Element, Enum, Table};

/// A state of db containing types and tables.
pub type State = (Vec<Enum>, Vec<Table>);

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

        db.query("UPDATE ergol SET migration = $1;", &[&current])
            .await?;

        current += 1;
    }

    Ok(())
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

/// A unit of diff between db states.
#[derive(Clone, Debug)]
pub enum DiffElement {
    /// A new element needs to be created.
    Create(Element),

    /// An element needs to be dropped.
    Drop(Element),

    /// An element needs to be changed.
    Alter(Element, Element),
}

impl DiffElement {
    /// Returns the hint of migration.
    pub fn hint(&self) -> String {
        match self {
            DiffElement::Create(e) => e.create(),
            DiffElement::Drop(e) => e.drop(),
            DiffElement::Alter(_, _) => String::from("-- need to do some manual stuff here"),
        }
    }

    /// Returns the hint to revert the migration.
    pub fn hint_revert(&self) -> String {
        match self {
            DiffElement::Create(e) => DiffElement::Drop(e.clone()).hint(),
            DiffElement::Drop(e) => DiffElement::Create(e.clone()).hint(),
            DiffElement::Alter(x, y) => DiffElement::Alter(y.clone(), x.clone()).hint(),
        }
    }
}

/// The diff elements between db states.
#[derive(Clone, Debug)]
pub struct Diff(Vec<DiffElement>);

impl Diff {
    /// Returns a hint of the migration request.
    pub fn hint(&self) -> String {
        self.0
            .iter()
            .map(DiffElement::hint)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Returns a hint of the revert migration request.
    pub fn hint_revert(&self) -> String {
        self.0
            .iter()
            .map(DiffElement::hint_revert)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Order the tables in the diff.
    pub fn order(self) -> Diff {
        self
    }
}

/// Computes the diff between two states.
pub fn diff((before_enums, before_tables): State, (after_enums, after_tables): State) -> Diff {
    let mut vec = vec![];

    for e in &before_enums {
        match after_enums.iter().find(|x| x.name == e.name) {
            None => vec.push(DiffElement::Drop(Element::Enum(e.clone()))),
            Some(x) => vec.push(DiffElement::Alter(
                Element::Enum(e.clone()),
                Element::Enum(x.clone()),
            )),
        }
    }

    for e in after_enums {
        if before_enums.iter().find(|x| x.name == e.name).is_none() {
            vec.push(DiffElement::Create(Element::Enum(e)));
        }
    }

    for e in &before_tables {
        match after_tables.iter().find(|x| x.name == e.name) {
            None => vec.push(DiffElement::Drop(Element::Table(e.clone()))),
            Some(x) if x != e => vec.push(DiffElement::Alter(
                Element::Table(e.clone()),
                Element::Table(x.clone()),
            )),
            _ => (),
        }
    }

    for e in after_tables {
        if before_tables.iter().find(|x| x.name == e.name).is_none() {
            vec.push(DiffElement::Create(Element::Table(e)));
        }
    }

    Diff(vec)
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
