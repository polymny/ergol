use crate::prelude::*;

use std::marker::{PhantomData, Sync};

use tokio_postgres::{types::ToSql, Error};

#[crate::async_trait::async_trait]
pub trait Query {
    type Output;
    async fn execute(&self, client: &tokio_postgres::Client) -> Result<Self::Output, Error>;
}

/// A filter on a request.
pub struct Filter {
    /// The name of the column.
    pub column: &'static str,

    /// The value for the filter.
    pub value: Box<dyn ToSql + Send + Sync + 'static>,

    /// The operator of the filter.
    pub operator: Operator,
}

/// A select query on T.
pub struct Select<T: ToTable + ?Sized> {
    _marker: PhantomData<T>,
    limit: Option<usize>,

    /// A filter.
    filter: Option<Filter>,
}

impl<T: ToTable + Sync> Select<T> {
    pub fn new() -> Select<T> {
        Select {
            _marker: PhantomData,
            limit: None,
            filter: None,
        }
    }

    pub fn limit(mut self, limit: usize) -> Select<T> {
        self.limit = Some(limit);
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
        }
    }
}

#[crate::async_trait::async_trait]
impl<T: ToTable + Sync> Query for Select<T> {
    type Output = Vec<T>;

    async fn execute(&self, client: &tokio_postgres::Client) -> Result<Self::Output, Error> {
        let query = format!(
            "SELECT * FROM {}{}{};",
            T::table_name(),
            if let Some(limit) = self.limit {
                format!(" LIMIT {}", limit)
            } else {
                String::new()
            },
            if let Some(filter) = self.filter.as_ref() {
                format!(" WHERE {} {} $1", filter.column, filter.operator.to_str())
            } else {
                String::new()
            }
        );

        if let Some(filter) = self.filter.as_ref() {
            Ok(client
                .query(&query as &str, &[&*filter.value])
                .await?
                .into_iter()
                .map(<T as ToTable>::from_row)
                .collect::<Vec<_>>())
        } else {
            Ok(client
                .query(&query as &str, &[])
                .await?
                .into_iter()
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

            async fn execute(
                &self,
                client: &tokio_postgres::Client,
            ) -> Result<Self::Output, Error> {
                for query in &self.0 {
                    client.query(query as &str, &[]).await?;
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
