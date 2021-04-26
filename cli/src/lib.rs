use std::env::current_dir;
use std::error::Error;
use std::fs::{copy, create_dir, read_dir, read_to_string, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use case::CaseExt;

use serde::{Deserialize, Serialize};

/// A state of db containing types and tables.
pub type State = (Vec<Enum>, Vec<Table>);

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
        if path.ends_with("json") {
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
    Ok((enums, tables))
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
            Some(x) => vec.push(DiffElement::Alter(
                Element::Table(e.clone()),
                Element::Table(x.clone()),
            )),
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

    Ok(())
}

/// An element that can be created in the db (can be a table or a type).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Element {
    /// An enum type.
    Enum(Enum),

    /// A table.
    Table(Table),
}

impl Element {
    /// Returns the create query of the element.
    pub fn create(&self) -> String {
        match self {
            Element::Enum(e) => e.create_type(),
            Element::Table(t) => t.create_table(),
        }
    }

    /// Returns the drop query of the element.
    pub fn drop(&self) -> String {
        match self {
            Element::Enum(e) => e.drop_type(),
            Element::Table(t) => t.drop_table(),
        }
    }
}

/// The struct that holds to information to create, drop or migrate an enum type.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Enum {
    /// The name of the type.
    pub name: String,

    /// The variants.
    pub variants: Vec<String>,
}

impl Enum {
    /// Creates the type.
    pub fn create_type(&self) -> String {
        format!(
            "CREATE TYPE {} AS ENUM ('{}');",
            self.name,
            self.variants.join("', '")
        )
    }

    /// Drops the type.
    pub fn drop_type(&self) -> String {
        format!("DROP TYPE {};", self.name)
    }
}

/// The struct that holds the information to create, drop or migrate a table.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Table {
    /// The name of the table.
    pub name: String,

    /// The columns of the table.
    pub columns: Vec<Column>,
}

impl Table {
    /// Creates a new empty table.
    pub fn new(name: &str) -> Table {
        Table {
            name: name.into(),
            columns: vec![],
        }
    }

    /// Returns the create table query for the table.
    pub fn create_table(&self) -> String {
        format!(
            "CREATE TABLE {} (\n    {}\n);",
            self.name,
            self.columns
                .iter()
                .map(|x| format!(
                    "{} {}{}",
                    x.name,
                    x.ty.to_postgres(),
                    if x.unique { " UNIQUE" } else { "" }
                ))
                .collect::<Vec<_>>()
                .join(",\n    ")
        )
    }

    /// Returns the drop table query for the table.
    pub fn drop_table(&self) -> String {
        format!("DROP TABLE {} CASCADE;", self.name)
    }
}

/// A column of a table.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Column {
    /// The name of the column.
    pub name: String,

    /// The type of the column.
    pub ty: Ty,

    /// Whether the column is unique or not.
    pub unique: bool,
}

impl Column {
    /// Creates a new column.
    pub fn new(name: &str, ty: Ty, unique: bool) -> Column {
        Column {
            name: name.into(),
            ty,
            unique,
        }
    }
}

/// The type of a column.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Ty {
    /// An ID column.
    Id,

    /// An i32 column.
    I32,

    /// A boolean column.
    Bool,

    /// A string column.
    String,

    /// A JSON value.
    Json,

    /// An optional type.
    Option(Box<Ty>),

    /// An enum type.
    Enum(String),

    /// A reference to another type.
    Reference(String),
}

impl Ty {
    /// Returns the postgres representation of the type.
    pub fn to_postgres(&self) -> String {
        match self {
            Ty::Id => "SERIAL PRIMARY KEY".to_owned(),
            Ty::String => "VARCHAR NOT NULL".to_owned(),
            Ty::I32 => "INT NOT NULL".to_owned(),
            Ty::Bool => "BOOL NOT NULL".to_owned(),
            Ty::Json => "JSON NOT NULL".to_owned(),
            Ty::Option(ty) => {
                let current = ty.to_postgres();
                debug_assert!(current.ends_with(" NOT NULL"));
                current[0..current.len() - 9].to_owned()
            }
            Ty::Enum(s) => format!("{} NOT NULL", s.to_snake()),
            Ty::Reference(s) => format!("INT NOT NULL REFERENCES {} (id)", s.to_snake()),
        }
    }
}

fn extract_chevrons(pattern: &str) -> Option<&str> {
    Some(pattern.split("<").nth(1)?.split(">").nth(0)?.trim())
}

impl FromStr for Ty {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "String" => return Ok(Ty::String),
            "i32" => return Ok(Ty::I32),
            "bool" => return Ok(Ty::Bool),
            _ => (),
        }

        if s.starts_with("Option <") {
            Self::from_str(extract_chevrons(s).ok_or(())?).map(|x| Ty::Option(Box::new(x)))
        } else if s.starts_with("Json <") {
            Ok(Ty::Json)
        } else {
            Ok(Ty::Enum(s.to_snake()))
        }
    }
}
