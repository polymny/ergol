use std::str::FromStr;

use case::CaseExt;

use serde::{Deserialize, Serialize};

/// An element that can be created in the db (can be a table or a type).
#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
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
