# One-to-one and many-to-one relationships

Let's say you want a user to be able to have projects. You can use the
`#[many_to_one]` attribute in order to do so. Let's take the following code as
an example:

```rust
# extern crate ergol;
use ergol::prelude::*;

#[ergol]
pub struct User {
    #[id] pub id: i32,
    #[unique] pub username: String,
    pub password: String,
    pub age: Option<i32>,
}

#[ergol]
pub struct Project {
    #[id] pub id: i32,
    pub name: String,
    #[many_to_one(projects)] pub owner: User,
}
```

Once you have defined this struct, many more functions become available:

```rust
# extern crate tokio;
# extern crate ergol;
# use ergol::prelude::*;
# #[ergol]
# pub struct User {
#     #[id] pub id: i32,
#     #[unique] pub username: String,
#     pub password: String,
#     pub age: Option<i32>,
# }
# #[ergol]
# pub struct Project {
#     #[id] pub id: i32,
#     pub name: String,
#     #[many_to_one(projects)] pub owner: User,
# }
# #[tokio::main]
# async fn main() -> Result<(), ergol::tokio_postgres::Error> {
#     let (db, connection) = ergol::connect(
#         "host=localhost user=ergol password=ergol dbname=ergol",
#         ergol::tokio_postgres::NoTls,
#     )
#     .await?;
#     tokio::spawn(async move {
#         if let Err(e) = connection.await {
#             eprintln!("connection error: {}", e);
#         }
#     });
// Drop the tables if they exist
Project::drop_table().execute(&db).await.ok();
User::drop_table().execute(&db).await.ok();

// Create the tables
User::create_table().execute(&db).await?;
Project::create_table().execute(&db).await?;

// Create two users and save them into the database
let thomas: User = User::create("thomas", "pa$$w0rd", 28).save(&db).await?;
User::create("nicolas", "pa$$w0rd", 28).save(&db).await?;

// Create some projects for the user
let project: Project = Project::create("My first project", &thomas).save(&db).await?;
Project::create("My second project", &thomas).save(&db).await?;

// You can easily find all projects from the user
let projects: Vec<Project> = thomas.projects(&db).await?;

// You can also find the owner of a project
let owner: User = projects[0].owner(&db).await?;
# Ok(())
# }
```

You can similarly have one-to-one relationship between a user and a project by
using the `#[one_to_one]` attribute:

```rust
# extern crate ergol;
# use ergol::prelude::*;
# #[ergol]
# pub struct User {
#     #[id] pub id: i32,
#     #[unique] pub username: String,
#     pub password: String,
#     pub age: Option<i32>,
# }
#[ergol]
pub struct Project {
    #[id] pub id: i32,
    pub name: String,
    #[one_to_one(project)] pub owner: User,
}
```

This will add the `UNIQUE` attribute in the database and make the `project`
method only return an option:

```rust
# extern crate tokio;
# extern crate ergol;
# use ergol::prelude::*;
# #[ergol]
# pub struct User {
#     #[id] pub id: i32,
#     #[unique] pub username: String,
#     pub password: String,
#     pub age: Option<i32>,
# }
# #[ergol]
# pub struct Project {
#     #[id] pub id: i32,
#     pub name: String,
#     #[one_to_one(project)] pub owner: User,
# }
# #[tokio::main]
# async fn main() -> Result<(), ergol::tokio_postgres::Error> {
#     let (db, connection) = ergol::connect(
#         "host=localhost user=ergol password=ergol dbname=ergol2",
#         ergol::tokio_postgres::NoTls,
#     )
#     .await?;
#     tokio::spawn(async move {
#         if let Err(e) = connection.await {
#             eprintln!("connection error: {}", e);
#         }
#     });
# Project::drop_table().execute(&db).await.ok();
# User::drop_table().execute(&db).await.ok();
# User::create_table().execute(&db).await?;
# Project::create_table().execute(&db).await?;
# let thomas: User = User::create("thomas", "pa$$w0rd", 28).save(&db).await?;
// You can easily find a user's project
let project: Option<Project> = thomas.project(&db).await?;
# Ok(())
# }
```

Note that that way, a project has exactly one owner, but a user can have no
project.
