# Ergol macro

The `#[ergol]` macro transforms your struct in a table, and gives you methods
to easily access it.

```rust
# extern crate ergol;
use ergol::prelude::*;

#[ergol]
pub struct User {
    #[id] pub id: i32,
    #[unique] pub username: String,
    pub password: String,
    pub age: i32,
}
```

## The `#[id]` attribute

In every table, a primary key is required. For the moment, `ergol` requires to
have an `id` column, which is an `i32` field named `id`.

## The `#[unique]` attribute

If a field is marked with the `#[unique]` attribute, the macro will generate
extra methods to fetch an element from this attribute. For example, with the
`#[unique] pub username: String` attribute, we can now use the following:

```rust
# extern crate tokio;
# extern crate ergol;
# use ergol::prelude::*;
# #[ergol]
# pub struct User {
#     #[id] pub id: i32,
#     #[unique] pub username: String,
#     pub password: String,
#     pub age: i32,
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
# User::drop_table().execute(&db).await.ok();
# User::create_table().execute(&db).await?;
let user: Option<User> = User::get_by_username("thomas", &db).await?;
# Ok(())
# }
```
