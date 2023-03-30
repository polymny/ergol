//! This crate contains all the necessary queries.

use crate::prelude::*;

use std::marker::{PhantomData, Sync};

use tokio_postgres::{types::ToSql, Error};

/// Any query should implement this trait.
#[crate::async_trait::async_trait]
pub trait Query {
    /// The output type of the query.
    type Output;

    /// Performs the query and returns a result.
    async fn execute(self, ergol: &Ergol) -> Result<Self::Output, Error>;
}

/// A filter on a request.
pub enum Filter {
    /// A filter from a binary operator.
    Binary {
        /// The name of the column.
        column: &'static str,

        /// The value for the filter.
        value: Box<dyn ToSql + Send + Sync + 'static>,

        /// The operator of the filter.
        operator: Operator,
    },

    /// And between two filters.
    And(Box<Filter>, Box<Filter>),

    /// Or between two filters
    Or(Box<Filter>, Box<Filter>),
}

impl Filter {
    /// Returns the sql representation of the filter.
    pub fn to_string<'a>(
        &'a self,
        first_index: i32,
    ) -> (String, i32, Vec<&'a (dyn ToSql + Sync + 'static)>) {
        match self {
            Filter::Binary {
                column,
                operator,
                value,
            } => (
                format!("\"{}\" {} ${}", column, operator.to_str(), first_index),
                first_index + 1,
                vec![value.as_ref()],
            ),
            Filter::And(a, b) => {
                let (a, next, mut args1) = a.to_string(first_index);
                let (b, next, args2) = b.to_string(next);
                args1.extend(args2);
                (format!("({} AND {})", a, b), next, args1)
            }
            Filter::Or(a, b) => {
                let (a, next, mut args1) = a.to_string(first_index);
                let (b, next, args2) = b.to_string(next);
                args1.extend(args2);
                (format!("({} OR {})", a, b), next, args1)
            }
        }
    }

    /// Returns another filter that performs an and between self and other.
    pub fn and(self, other: Filter) -> Filter {
        Filter::And(Box::new(self), Box::new(other))
    }

    /// Returns another filter that performs an or between self or other.
    pub fn or(self, other: Filter) -> Filter {
        Filter::Or(Box::new(self), Box::new(other))
    }
}

/// Decend of ascend.
#[derive(Copy, Clone)]
pub enum Order {
    /// Ascending order.
    Ascend,

    /// Descending order.
    Descend,
}

impl Order {
    /// Convers the order to a string.
    pub fn to_str(self) -> &'static str {
        match self {
            Order::Ascend => "ASC",
            Order::Descend => "DESC",
        }
    }
}

/// An order for a request.
pub struct OrderBy {
    /// The name of the column.
    pub column: &'static str,

    /// The type of order.
    pub order: Order,
}

/// A select query on T.
pub struct Select<T: ToTable + ?Sized> {
    _marker: PhantomData<T>,

    /// How many results you want to have.
    limit: Option<usize>,

    /// The offset of the request.
    offset: Option<usize>,

    /// The order of the request.
    order_by: Option<OrderBy>,

    /// A filter.
    filter: Option<Filter>,
}

impl<T: ToTable + Sync> Select<T> {
    /// Creates a new select query with no limit.
    pub fn new() -> Select<T> {
        Select {
            _marker: PhantomData,
            limit: None,
            offset: None,
            order_by: None,
            filter: None,
        }
    }

    /// Sets the limit of the select query.
    pub fn limit(mut self, limit: usize) -> Select<T> {
        self.limit = Some(limit);
        self
    }

    /// Sets the offset on the select query.
    pub fn offset(mut self, offset: usize) -> Select<T> {
        self.offset = Some(offset);
        self
    }

    /// Sets the order by of the select query.
    pub fn order_by(mut self, order_by: OrderBy) -> Select<T> {
        self.order_by = Some(order_by);
        self
    }

    /// Sets the filter of the select query.
    pub fn filter(mut self, filter: Filter) -> Select<T> {
        self.filter = Some(filter);
        self
    }
}

/// The different comparison operators for filters.
#[derive(Copy, Clone)]
pub enum Operator {
    /// Are equals
    Eq,

    /// Is greater or equal.
    Geq,

    /// Is lesser or equal.
    Leq,

    /// Is greater than.
    Gt,

    /// Is lesser than.
    Lt,

    /// Is different.
    Neq,

    /// String like another string.
    Like,

    /// String similary to another string.
    SimilarTo,
}

impl Operator {
    /// Converts the operator in the postgres format.
    pub fn to_str(self) -> &'static str {
        match self {
            Operator::Eq => "=",
            Operator::Geq => ">=",
            Operator::Leq => "<=",
            Operator::Gt => ">",
            Operator::Lt => "<",
            Operator::Neq => "!=",
            Operator::Like => "LIKE",
            Operator::SimilarTo => "SIMILAR TO",
        }
    }
}

#[crate::async_trait::async_trait]
impl<T: ToTable + Sync> Query for Select<T> {
    type Output = Vec<T>;

    async fn execute(self, ergol: &Ergol) -> Result<Self::Output, Error> {
        let filter = self.filter.as_ref().map(|x| x.to_string(1));

        let query = format!(
            "SELECT * FROM \"{}\"{}{}{}{};",
            T::table_name(),
            if let Some((filter, _, _)) = filter.as_ref() {
                format!(" WHERE {}", filter)
            } else {
                String::new()
            },
            if let Some(order_by) = self.order_by.as_ref() {
                format!(
                    " ORDER BY \"{}\" {}",
                    order_by.column,
                    order_by.order.to_str()
                )
            } else {
                String::new()
            },
            if let Some(limit) = self.limit {
                format!(" LIMIT {}", limit)
            } else {
                String::new()
            },
            if let Some(offset) = self.offset {
                format!(" OFFSET {}", offset)
            } else {
                String::new()
            }
        );

        if let Some((_, _, args)) = filter {
            Ok(ergol
                .client
                .query(&query as &str, &args[..])
                .await?
                .iter()
                .map(<T as ToTable>::from_row)
                .collect::<Vec<_>>())
        } else {
            Ok(ergol
                .client
                .query(&query as &str, &[])
                .await?
                .iter()
                .map(<T as ToTable>::from_row)
                .collect::<Vec<_>>())
        }
    }
}

macro_rules! make_string_query {
    ($i: ident) => {
        pub struct $i(pub Vec<String>);

        impl $i {
            pub fn single(s: String) -> $i {
                $i(vec![s])
            }
        }

        #[crate::async_trait::async_trait]
        impl Query for $i {
            type Output = ();

            async fn execute(self, ergol: &Ergol) -> Result<Self::Output, Error> {
                for query in &self.0 {
                    ergol.client.query(query as &str, &[]).await?;
                }
                Ok(())
            }
        }
    };
}

make_string_query!(CreateTable);
make_string_query!(DropTable);
make_string_query!(CreateType);
make_string_query!(DropType);
