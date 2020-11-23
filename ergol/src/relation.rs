use std::marker::PhantomData;

use bytes::BytesMut;

use tokio_postgres::types::{to_sql_checked, FromSql, IsNull, ToSql, Type};

use crate::{pg::Pg, ToTable};

pub trait Relation<U: ToTable> {
    type Target;
    type Reverse;
    fn from_rows(rows: Vec<tokio_postgres::Row>) -> Self::Reverse;
}

#[derive(Debug, Clone, Copy)]
pub struct OneToOne<T: ToTable> {
    _phantom: PhantomData<T>,
    id: i32,
}

impl<T: ToTable> OneToOne<T> {
    pub fn new(id: i32) -> OneToOne<T> {
        OneToOne {
            _phantom: PhantomData,
            id,
        }
    }

    pub async fn fetch(&self, client: &tokio_postgres::Client) -> Result<T, tokio_postgres::Error> {
        let query = format!(
            "SELECT * FROM {} WHERE {} = $1",
            T::table_name(),
            T::id_name()
        );
        let mut rows = client.query(&query as &str, &[&self.id]).await?;
        let row = rows.pop().unwrap();
        Ok(<T as ToTable>::from_row(row))
    }
}

impl<T: ToTable, U: ToTable> Relation<U> for OneToOne<T> {
    type Target = T;
    type Reverse = Option<U>;

    fn from_rows(mut rows: Vec<tokio_postgres::Row>) -> Self::Reverse {
        rows.pop().map(<U as ToTable>::from_row)
    }
}

impl<T: ToTable> Pg for OneToOne<T> {
    fn ty() -> String {
        format!(
            "INT UNIQUE NOT NULL REFERENCES {} ({})",
            T::table_name(),
            T::id_name(),
        )
    }
}

impl<T: ToTable> From<T> for OneToOne<T> {
    fn from(t: T) -> OneToOne<T> {
        OneToOne::new(t.id())
    }
}

impl<T: ToTable> From<&T> for OneToOne<T> {
    fn from(t: &T) -> OneToOne<T> {
        OneToOne::new(t.id())
    }
}

impl<'a, T: ToTable> FromSql<'a> for OneToOne<T> {
    fn from_sql(
        ty: &Type,
        raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Sync + Send>> {
        Ok(OneToOne::new(i32::from_sql(ty, raw)?))
    }

    fn accepts(ty: &Type) -> bool {
        <i32 as FromSql>::accepts(ty)
    }
}

impl<T: ToTable> ToSql for OneToOne<T> {
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + 'static + Sync + Send>> {
        self.id.to_sql(ty, out)
    }

    fn accepts(ty: &Type) -> bool {
        <i32 as ToSql>::accepts(ty)
    }

    to_sql_checked!();
}

#[derive(Debug, Copy, Clone)]
pub struct ManyToOne<T: ToTable> {
    _phantom: PhantomData<T>,
    id: i32,
}

impl<T: ToTable> ManyToOne<T> {
    pub fn new(id: i32) -> ManyToOne<T> {
        ManyToOne {
            _phantom: PhantomData,
            id,
        }
    }

    pub async fn fetch(&self, client: &tokio_postgres::Client) -> Result<T, tokio_postgres::Error> {
        let query = format!(
            "SELECT * FROM {} WHERE {} = $1",
            T::table_name(),
            T::id_name()
        );
        let mut rows = client.query(&query as &str, &[&self.id]).await?;
        let row = rows.pop().unwrap();
        Ok(<T as ToTable>::from_row(row))
    }
}

impl<T: ToTable, U: ToTable> Relation<U> for ManyToOne<T> {
    type Target = T;
    type Reverse = Vec<U>;
    fn from_rows(rows: Vec<tokio_postgres::Row>) -> Self::Reverse {
        rows.into_iter().map(<U as ToTable>::from_row).collect()
    }
}

impl<T: ToTable> Pg for ManyToOne<T> {
    fn ty() -> String {
        format!(
            "INT NOT NULL REFERENCES {} ({})",
            T::table_name(),
            T::id_name(),
        )
    }
}

impl<'a, T: ToTable> FromSql<'a> for ManyToOne<T> {
    fn from_sql(
        ty: &Type,
        raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Sync + Send>> {
        Ok(ManyToOne::new(i32::from_sql(ty, raw)?))
    }

    fn accepts(ty: &Type) -> bool {
        <i32 as FromSql>::accepts(ty)
    }
}

impl<T: ToTable> ToSql for ManyToOne<T> {
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + 'static + Sync + Send>> {
        self.id.to_sql(ty, out)
    }

    fn accepts(ty: &Type) -> bool {
        <i32 as ToSql>::accepts(ty)
    }

    to_sql_checked!();
}

impl<T: ToTable> From<T> for ManyToOne<T> {
    fn from(t: T) -> ManyToOne<T> {
        ManyToOne::new(t.id())
    }
}

impl<T: ToTable> From<&T> for ManyToOne<T> {
    fn from(t: &T) -> ManyToOne<T> {
        ManyToOne::new(t.id())
    }
}
