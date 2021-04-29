//! This module contains everything needed to compute diffs between databases.

use ergol_core::{Column, Element, Enum, Table};

/// A state of db containing types and tables.
pub type State = (Vec<Enum>, Vec<Table>);

/// A unit of diff between db states.
#[derive(Clone, Debug)]
pub enum DiffElement {
    /// A new element needs to be created.
    Create(Element),

    /// An element needs to be dropped.
    Drop(Element),

    /// Creates a new column in a table.
    CreateColumn(String, Column),

    /// Drops a column in a table.
    DropColumn(String, Column),

    /// An element needs to be changed.
    Alter(Element, Element),
}

impl DiffElement {
    /// Returns the hint of migration.
    pub fn hint(&self) -> String {
        match self {
            DiffElement::Create(e) => e.create(),
            DiffElement::Drop(e) => e.drop(),
            DiffElement::CreateColumn(t, c) => {
                format!(
                    "ALTER TABLE {} ADD {} {} DEFAULT -- TODO insert default type here",
                    t,
                    c.name,
                    c.ty.to_postgres(),
                )
            }
            DiffElement::DropColumn(t, c) => format!("ALTER TABLE {} DROP COLUMN {};", t, c.name),
            DiffElement::Alter(_, _) => String::from("-- need to do some manual stuff here"),
        }
    }

    /// Returns the hint to revert the migration.
    pub fn hint_revert(&self) -> String {
        match self {
            DiffElement::Create(e) => DiffElement::Drop(e.clone()).hint(),
            DiffElement::Drop(e) => DiffElement::Create(e.clone()).hint(),
            DiffElement::CreateColumn(c, t) => DiffElement::DropColumn(c.clone(), t.clone()).hint(),
            DiffElement::DropColumn(c, t) => DiffElement::CreateColumn(c.clone(), t.clone()).hint(),
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
            Some(x) if x != e => vec.push(DiffElement::Alter(
                Element::Enum(e.clone()),
                Element::Enum(x.clone()),
            )),
            _ => (),
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
            Some(x) if x != e => vec.append(&mut diff_table(e, x)),
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

/// Computes the diff between two tables.
pub fn diff_table(before: &Table, after: &Table) -> Vec<DiffElement> {
    let mut vec = vec![];

    for c in &before.columns {
        match after.columns.iter().find(|x| x.name == c.name) {
            None => vec.push(DiffElement::DropColumn(before.name.clone(), c.clone())),
            Some(c2) if c != c2 => eprintln!("should alter column"),
            _ => (),
        }
    }

    for c in &after.columns {
        if before.columns.iter().find(|x| x.name == c.name).is_none() {
            vec.push(DiffElement::CreateColumn(before.name.clone(), c.clone()));
        }
    }

    vec
}
