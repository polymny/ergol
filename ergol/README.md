# ergol

[![CI](https://github.com/polymny/ergol/workflows/build/badge.svg?branch=master&event=push)](https://github.com/polymny/ergol/actions?query=workflow%3Abuild) [![Docs](https://docs.rs/ergol/badge.svg)](https://docs.rs/ergol/) [Book](https://ergol-rs.github.io)

This crate provides the `#[ergol]` macro. It allows to persist the data in a
database. For example, you just have to write

```rust
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
// Drop the user table if it exists
User::drop_table().execute(&client).await.ok();

// Create the user table
User::create_table().execute(&client).await?;

// Create a user and save it into the database
let mut user: User = User::create("thomas", "pa$$w0rd", Some(28)).save(&client).await?;

// Change some of its fields
*user.age.as_mut().unwrap() += 1;

// Update the user in the database
user.save(&client).await?;

// Fetch a user by its username thanks to the unique attribute
let user: Option<User> = User::get_by_username("thomas", &client).await?;

// Select all users
let users: Vec<User> = User::select().execute(&client).await?;
```

See [the book](https://ergol-rs.github.io) for more information.
