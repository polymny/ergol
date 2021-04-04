//! [![CI](https://github.com/polymny/ergol/workflows/build/badge.svg?branch=master&event=push)](https://github.com/polymny/ergol/actions?query=workflow%3Abuild)
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
//! # Many-to-one and one-to-one relationships
//!
//! Let's say you want a user to be able to have projects. You can use the
//! `#[many_to_one]` attribute in order to do so. Just add:
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
//! #[ergol]
//! pub struct Project {
//!     #[id] pub id: i32,
//!     pub name: String,
//!     #[many_to_one(projects)] pub owner: User,
//! }
//! ```
//!
//! Once you have defined this struct, many more functions become available:
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
//! # #[ergol]
//! # pub struct Project {
//! #     #[id] pub id: i32,
//! #     pub name: String,
//! #     #[many_to_one(projects)] pub owner: User,
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
//! Project::drop_table().execute(&client).await.ok();
//! User::drop_table().execute(&client).await.ok();
//!
//! // Create the user table
//! User::create_table().execute(&client).await?;
//! Project::create_table().execute(&client).await?;
//!
//! // Create two users and save them into the database
//! let thomas: User = User::create("thomas", "pa$$w0rd", 28).save(&client).await?;
//! User::create("nicolas", "pa$$w0rd", 28).save(&client).await?;
//!
//! // Create some projects for the user
//! let project: Project = Project::create("My first project", &thomas).save(&client).await?;
//! Project::create("My second project", &thomas).save(&client).await?;
//!
//! // You can easily find all projects from the user
//! let projects: Vec<Project> = thomas.projects(&client).await?;
//!
//! // You can also find the owner of a project
//! let owner: User = projects[0].owner(&client).await?;
//! # Ok(())
//! # }
//! ```
//!
//! You can similarly have one-to-one relationship between a user and a project by
//! using the `#[one_to_one]` attribute:
//!
//! ```rust
//! # use ergol::prelude::*;
//! # #[ergol]
//! # pub struct User {
//! #     #[id] pub id: i32,
//! #     #[unique] pub username: String,
//! #     pub password: String,
//! # }
//! #[ergol]
//! pub struct Project {
//!     #[id] pub id: i32,
//!     pub name: String,
//!     #[one_to_one(project)] pub owner: User,
//! }
//! ```
//!
//! This will add the `UNIQUE` attribute in the database and make the `project`
//! method only return an option:
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
//! # #[ergol]
//! # pub struct Project {
//! #     #[id] pub id: i32,
//! #     pub name: String,
//! #     #[one_to_one(project)] pub owner: User,
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
//! # Project::drop_table().execute(&client).await.ok();
//! # User::drop_table().execute(&client).await.ok();
//! # User::create_table().execute(&client).await?;
//! # Project::create_table().execute(&client).await?;
//! # let thomas: User = User::create("thomas", "pa$$w0rd", 28).save(&client).await?;
//! // You can easily find a user's project
//! let project: Option<Project> = thomas.project(&client).await?;
//! # Ok(())
//! # }
//! ```
//!
//! Note that that way, a project has exactly one owner, but a user can have no
//! project.
//!
//! # Many-to-many relationships
//!
//! This macro also supports many-to-many relationships. In order to do so, you
//! need to use the `#[many_to_many]` attribute:
//!
//! ```rust
//! # use ergol::prelude::*;
//! # #[ergol]
//! # pub struct User {
//! #     #[id] pub id: i32,
//! #     #[unique] pub username: String,
//! #     pub password: String,
//! # }
//! #[ergol]
//! pub struct Project {
//!     #[id] pub id: i32,
//!     pub name: String,
//!     #[many_to_many(visible_projects)] pub authorized_users: User,
//! }
//! ```
//!
//! The same way, you will have plenty of functions that you will be able to use to
//! manage your objects:
//!
//! ```rust
//! # use ergol::prelude::*;
//! # #[ergol]
//! # pub struct User {
//! #     #[id] pub id: i32,
//! #     #[unique] pub username: String,
//! #     pub password: String,
//! #     pub age: i32,
//! # }
//! # #[ergol]
//! # pub struct Project {
//! #     #[id] pub id: i32,
//! #     pub name: String,
//! #     #[many_to_many(visible_projects)] pub authorized_users: User,
//! # }
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
//! # Project::drop_table().execute(&client).await.ok();
//! # User::drop_table().execute(&client).await.ok();
//! # User::create_table().execute(&client).await?;
//! # Project::create_table().execute(&client).await?;
//! # User::create("thomas", "pa$$w0rd", 28).save(&client).await?;
//! # User::create("nicolas", "pa$$w0rd", 28).save(&client).await?;
//! // Find some users in the database
//! let thomas = User::get_by_username("thomas", &client).await?.unwrap();
//! let nicolas = User::get_by_username("nicolas", &client).await?.unwrap();
//!
//! // Create a project
//! let first_project = Project::create("My first project").save(&client).await?;
//!
//! // Thomas can access this project
//! first_project.add_authorized_user(&thomas, &client).await?;
//!
//! // The other way round
//! nicolas.add_visible_project(&first_project, &client).await?;
//!
//! // The second project can only be used by thomas
//! let second_project = Project::create("My second project").save(&client).await?;
//! thomas.add_visible_project(&second_project, &client).await?;
//!
//! // The third project can only be used by nicolas.
//! let third_project = Project::create("My third project").save(&client).await?;
//! third_project.add_authorized_user(&nicolas, &client).await?;
//!
//! // You can easily retrieve all projects available for a certain user
//! let projects: Vec<Project> = thomas.visible_projects(&client).await?;
//!
//! // And you can easily retrieve all users that have access to a certain project
//! let users: Vec<User> = first_project.authorized_users(&client).await?;
//!
//! // You can easily remove a user from a project
//! let _: bool = first_project.remove_authorized_user(&thomas, &client).await?;
//!
//! // Or vice-versa
//! let _: bool = nicolas.remove_visible_project(&first_project, &client).await?;
//!
//! // The remove functions return true if they successfully removed something.
//! # Ok(())
//! # }
//! ```
//!
//! ## Extra information in a many to many relationship
//!
//! It is possible to insert some extra information in a many to many relationship. The following
//! exemple gives roles for the users for projects.
//!
//! ```rust
//! # use ergol::prelude::*;
//! #[ergol]
//! pub struct User {
//!     #[id] pub id: i32,
//!     #[unique] pub username: String,
//!     pub password: String,
//! }
//!
//! #[derive(PgEnum, Debug)]
//! pub enum Role {
//!    Admin,
//!    Write,
//!    Read,
//! }
//!
//! #[ergol]
//! pub struct Project {
//!     #[id] pub id: i32,
//!     pub name: String,
//!     #[many_to_many(projects, Role)] pub users: User,
//! }
//! ```
//!
//! With these structures, the signature of generated methods change to take a role as argument,
//! and to return tuples of (User, Role) or (Project, Role).
//!
//! ```rust
//! # use ergol::prelude::*;
//! # #[ergol]
//! # pub struct User {
//! #     #[id] pub id: i32,
//! #     #[unique] pub username: String,
//! # }
//! # #[derive(PgEnum, Debug)]
//! # pub enum Role {
//! #    Admin,
//! #    Write,
//! #    Read,
//! # }
//! # #[ergol]
//! # pub struct Project {
//! #     #[id] pub id: i32,
//! #     pub name: String,
//! #     #[many_to_many(projects, Role)] pub users: User,
//! # }
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
//! # // Try to delete the database
//! # User::drop_table().execute(&client).await.ok();
//! # Project::drop_table().execute(&client).await.ok();
//! # Role::drop_type().execute(&client).await.ok();
//! # // Create the tables
//! # Role::create_type().execute(&client).await?;
//! # User::create_table().execute(&client).await?;
//! # Project::create_table().execute(&client).await?;
//! # User::create("tforgione").save(&client).await?;
//! # User::create("graydon").save(&client).await?;
//! let tforgione = User::get_by_username("tforgione", &client).await?.unwrap();
//! let graydon = User::get_by_username("graydon", &client).await?.unwrap();
//! let project = Project::create("My first project").save(&client).await?;
//! project.add_user(&tforgione, Role::Admin, &client).await?;
//! graydon.add_project(&project, Role::Read, &client).await?;
//!
//! for (user, role) in project.users(&client).await? {
//!     println!("{} has {:?} rights on project {:?}", user.username, role, project.name);
//! }
//!
//! let project = Project::create("My second project").save(&client).await?;
//! project.add_user(&tforgione, Role::Admin, &client).await?;
//!
//! let project = Project::create("My third project").save(&client).await?;
//! project.add_user(&graydon, Role::Admin, &client).await?;
//! project.add_user(&tforgione, Role::Read, &client).await?;
//!
//! for (project, role) in tforgione.projects(&client).await? {
//!     println!("{} has {:?} rights on project {:?}", tforgione.username, role, project.name);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Limitations
//!
//! For the moment, we still have plenty of limitations:
//!
//!   - this crate only works with tokio-postgres
//!   - there is no support for migrations
//!   - the names of the structs you use in `#[ergol]` must be used previously, e.g.
//!     ```rust,ignore
//!     mod user {
//!         use ergol::prelude::*;
//!
//!         #[ergol]
//!         pub struct User {
//!             #[id] pub id: i32,
//!         }
//!     }
//!
//!     use ergol::prelude::*;
//!     #[ergol]
//!     pub struct Project {
//!         #[id] pub id: i32,
//!         #[many_to_one(projects)] pub owner: user::User, // this will not work
//!     }
//!     ```
//!
//!     ```rust
//!     mod user {
//!         use ergol::prelude::*;
//!         #[ergol]
//!         pub struct User {
//!             #[id] pub id: i32,
//!         }
//!     }
//!     use user::User;
//!
//!     use ergol::prelude::*;
//!     #[ergol]
//!     pub struct Project {
//!         #[id] pub id: i32,
//!         #[many_to_one(projects)] pub owner: User, // this will work
//!     }
//!     ```

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
