//! [![CI](https://github.com/polymny/ergol/workflows/build/badge.svg?branch=master&event=push)](https://github.com/polymny/ergol/actions?query=workflow%3Abuild) [![Docs](https://docs.rs/ergol/badge.svg)](https://docs.rs/ergol/) [Book](https://ergol-rs.github.io)
//!
//! This crate provides the `#[ergol]` macro. It allows to persist the data in a
//! database. For example, you just have to write
//!
//! ```rust
//! use ergol::prelude::*;
//!
//! #[ergol]
//! pub struct User {
//!     #[id] pub id: i32,
//!     #[unique] pub username: String,
//!     pub password: String,
//!     pub age: Option<i32>,
//! }
//! ```
//!
//! and the `#[ergol]` macro will generate most of the code you will need. You'll
//! then be able to run code like the following:
//!
//! ```rust
//! # use ergol::prelude::*;
//! # #[ergol]
//! # pub struct User {
//! #     #[id] pub id: i32,
//! #     #[unique] pub username: String,
//! #     pub password: String,
//! #     pub age: Option<i32>,
//! # }
//! # use ergol::tokio;
//! # #[tokio::main]
//! # async fn main() -> Result<(), ergol::tokio_postgres::Error> {
//! #     let (client, connection) = ergol::connect(
//! #         "host=localhost user=ergol password=ergol dbname=ergol",
//! #         ergol::tokio_postgres::NoTls,
//! #     )
//! #     .await?;
//! #     tokio::spawn(async move {
//! #         if let Err(e) = connection.await {
//! #             eprintln!("connection error: {}", e);
//! #         }
//! #     });
//! // Drop the user table if it exists
//! User::drop_table().execute(&client).await.ok();
//!
//! // Create the user table
//! User::create_table().execute(&client).await?;
//!
//! // Create a user and save it into the database
//! let mut user: User = User::create("thomas", "pa$$w0rd", Some(28)).save(&client).await?;
//!
//! // Change some of its fields
//! *user.age.as_mut().unwrap() += 1;
//!
//! // Update the user in the database
//! user.save(&client).await?;
//!
//! // Fetch a user by its username thanks to the unique attribute
//! let user: Option<User> = User::get_by_username("thomas", &client).await?;
//!
//! // Select all users
//! let users: Vec<User> = User::select().execute(&client).await?;
//! # Ok(())
//! # }
//! ```
//!
//! See [the book](ergol-rs.github.io) for more information.

pub mod pg;
pub mod query;
pub mod relation;

use crate::query::{CreateTable, DropTable, Select};

/// Any type that should be transformed into a table should implement this trait.
///
/// You should not implement this trait yourself, and use the #[ergol] macro to implement this
/// trait for your structs.
#[async_trait::async_trait]
pub trait ToTable: Send + std::fmt::Debug + Sized {
    /// Converts a row of a table into an object.
    fn from_row_with_offset(row: &tokio_postgres::Row, offset: usize) -> Self;

    /// Converts a row of a table into an object.
    fn from_row(row: &tokio_postgres::Row) -> Self {
        Self::from_row_with_offset(row, 0)
    }

    /// Returns the name of the table corresponding to Self.
    fn table_name() -> &'static str;

    /// Returns the name of the primary key of the table corresponding to Self.
    fn id_name() -> &'static str;

    /// Returns the id of self.
    fn id(&self) -> i32;

    /// Returns the query that creates the table.
    fn create_table() -> CreateTable;

    /// Returns the query that drops the table.
    fn drop_table() -> DropTable;

    /// Returns a select query.
    fn select() -> Select<Self>;
}

pub use async_trait;
pub use bytes;
pub use tokio;
pub use tokio_postgres;

pub use ergol_proc_macro::ergol;

/// Any enum that has no field on any variant can derive `PgEnum` in order to be usable in a
/// `#[ergol]` struct.
///
/// # Note:
/// Any enum needs to derive Debug in order to derive PgEnum, since deriving Debug is
/// required in order to implement ToSql.
///
/// ```
/// # use ergol::prelude::*;
/// #[ergol]
/// pub struct MyStruct {
///     #[id] pub id: i32,
///     pub ok: IsOk,
/// }
///
/// #[derive(PgEnum, Debug)]
/// pub enum IsOk {
///     IAmOk,
///     IAmNotOk,
/// }
/// ```
pub use ergol_proc_macro::PgEnum;

/// The prelude contains the macros and usefull traits.
pub mod prelude {
    pub use crate::pg::Pg;
    pub use crate::query::Query;
    pub use crate::{ergol, Ergol, PgEnum, ToTable};
}

use tokio_postgres::{tls::MakeTlsConnect, Connection, Error, Socket};

/// The type that wraps the connection to the database.
pub struct Ergol {
    /// The connection to the postgres client.
    pub client: tokio_postgres::Client,
}

/// Connects to the specified database.
pub async fn connect<T: MakeTlsConnect<Socket>>(
    config: &str,
    tls: T,
) -> Result<(Ergol, Connection<Socket, T::Stream>), Error> {
    let (a, b) = tokio_postgres::connect(config, tls).await?;
    Ok((Ergol { client: a }, b))
}

#[cfg(feature = "with-rocket")]
pub mod pool {
    use crate::tokio_postgres::NoTls;
    use crate::{connect, Ergol, Error};
    use async_trait::async_trait;

    /// For dealing with database connection pools.
    pub struct Manager {
        url: String,
    }

    impl Manager {
        /// Creates a new manager from a new connection pool.
        pub fn new(url: &str) -> Manager {
            Manager {
                url: url.to_string(),
            }
        }
    }

    /// Creates a new connection pool.
    pub fn pool(url: &str, connections: usize) -> Pool {
        Pool::new(Manager::new(url), connections)
    }

    #[async_trait]
    impl deadpool::managed::Manager<Ergol, Error> for Manager {
        async fn create(&self) -> Result<Ergol, Error> {
            let (client, connection) = connect(&self.url, NoTls).await?;

            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("connection error: {}", e);
                }
            });

            Ok(client)
        }

        async fn recycle(&self, _conn: &mut Ergol) -> deadpool::managed::RecycleResult<Error> {
            Ok(())
        }
    }

    /// A database connection pool.
    pub type Pool = deadpool::managed::Pool<Ergol, Error>;
}

#[cfg(feature = "with-rocket")]
pub use pool::{pool, Pool};

#[cfg(feature = "with-rocket")]
pub use deadpool;
