use crate::prelude::*;

use std::marker::{PhantomData, Sync};

use tokio_postgres::Error;

#[crate::async_trait::async_trait]
pub trait Query {
    type Output;
    async fn execute(&self, client: &tokio_postgres::Client) -> Result<Self::Output, Error>;
}

pub struct Select<T: ToTable + ?Sized> {
    _marker: PhantomData<T>,
    limit: Option<usize>,
}

impl<T: ToTable + Sync> Select<T> {
    pub fn new() -> Select<T> {
        Select {
            _marker: PhantomData,
            limit: None,
        }
    }

    pub fn limit(mut self, limit: usize) -> Select<T> {
        self.limit = Some(limit);
        self
    }
}

#[crate::async_trait::async_trait]
impl<T: ToTable + Sync> Query for Select<T> {
    type Output = Vec<T>;

    async fn execute(&self, client: &tokio_postgres::Client) -> Result<Self::Output, Error> {
        let query = format!(
            "SELECT * FROM {}{};",
            T::table_name(),
            if let Some(limit) = self.limit {
                format!(" LIMIT {}", limit)
            } else {
                String::new()
            }
        );

        Ok(client
            .query(&query as &str, &[])
            .await?
            .into_iter()
            .map(<T as ToTable>::from_row)
            .collect::<Vec<_>>())
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
