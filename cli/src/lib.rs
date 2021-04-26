use std::env::current_dir;
use std::error::Error;
use std::fs::{copy, create_dir, read_dir, read_to_string, File};
use std::io::Write;
use std::path::{Path, PathBuf};

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

/// The diff elements between db states.
#[derive(Clone, Debug)]
pub struct Diff(Vec<DiffElement>);

impl Diff {
    /// Returns a hint of the migration request.
    pub fn hint(&self) -> String {
        self.0
            .iter()
            .map(|x| match x {
                DiffElement::Create(c) => c.create(),
                DiffElement::Drop(d) => d.drop(),
                DiffElement::Alter(_, _) => String::from("-- yeah have fun with that"),
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Returns a hint of the revert migration request.
    pub fn hint_drop(&self) -> String {
        self.0
            .iter()
            .map(|x| match x {
                DiffElement::Create(c) => c.drop(),
                DiffElement::Drop(d) => d.create(),
                DiffElement::Alter(_, _) => String::from("-- yeah have fun with that"),
            })
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
    file.write_all(diff.hint_drop().as_bytes())?;

    Ok(())
}
