use std::str::FromStr;

use case::CaseExt;

use serde::{Deserialize, Serialize};

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
                .map(|x| format!("{} {}", x.name, x.ty.to_postgres()))
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
}

impl Column {
    /// Creates a new column.
    pub fn new(name: &str, ty: Ty) -> Column {
        Column {
            name: name.into(),
            ty,
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

impl FromStr for Ty {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match dbg!(s) {
            "String" => return Ok(Ty::String),
            "i32" => return Ok(Ty::I32),
            "bool" => return Ok(Ty::Bool),
            _ => (),
        }

        if s.starts_with("Option <") {
            Self::from_str(
                s.split("<")
                    .nth(1)
                    .ok_or(())?
                    .split(">")
                    .nth(0)
                    .ok_or(())?
                    .trim(),
            )
            .map(|x| Ty::Option(Box::new(x)))
        } else {
            Ok(Ty::Enum(s.to_snake()))
        }
    }
}
