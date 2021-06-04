# Getting started

This crate provides the `#[ergol]` macro. You can use it by adding
```toml
ergol = "0.1"
```
to your dependencies.

It allows to persist the data in a database. For example, you just have to
write

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
```

and the `#[ergol]` macro will generate most of the code you will need. You'll
then be able to run code like the following:

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
// Drop the user table if it exists
User::drop_table().execute(&db).await.ok();

// Create the user table
User::create_table().execute(&db).await?;

// Create a user and save it into the database
let mut user: User = User::create("thomas", "pa$$w0rd", Some(28)).save(&db).await?;

// Change some of its fields
*user.age.as_mut().unwrap() += 1;

// Update the user in the database
user.save(&db).await?;

// Fetch a user by its username thanks to the unique attribute
let user: Option<User> = User::get_by_username("thomas", &db).await?;

// Select all users
let users: Vec<User> = User::select().execute(&db).await?;
# Ok(())
# }
```
