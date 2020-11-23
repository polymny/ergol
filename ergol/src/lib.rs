pub mod pg;
pub mod query;
pub mod relation;

use crate::query::{CreateTable, DropTable, Select};

#[async_trait::async_trait]
pub trait ToTable: Send + std::fmt::Debug {
    fn from_row(row: tokio_postgres::Row) -> Self;
    fn table_name() -> &'static str;
    fn id_name() -> &'static str;
    fn id(&self) -> i32;
    fn create_table() -> CreateTable;
    fn drop_table() -> DropTable;
    fn select() -> Select<Self>;
}

pub use async_trait;
pub use bytes;
pub use tokio;
pub use tokio_postgres;

pub mod prelude {
    pub use crate::pg::Pg;
    pub use crate::query::Query;
    pub use crate::ToTable;
    pub use ergol_proc_macro::{ergol, PgEnum};
}
