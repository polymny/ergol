# ergol: *an async ORM in Rust (WIP)*

[![CI](https://github.com/polymny/ergol/workflows/build/badge.svg?branch=master&event=push)](https://github.com/polymny/ergol/actions?query=workflow%3Abuild)

This crate provides the `#[ergol]` macro.  It allows to persist the data in a
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
let mut user: User = User::create("thomas", "pa$$word", Some(28)).save(&client).await?;

// Change some of its fields
if let Some(age) = user.age.as_mut() {
    *age += 1;
}

// Update the user in the database
user.save(&client).await?;

// Fetch a user by its username thanks to the unique attribute
let user: Option<User> = User::get_by_username("thomas", &client)?;

// Select all users
let users: Vec<User> = User::select().execute(&client).await?;
```

## Many-to-one and one-to-one relationships

Let's say you want a user to be able to have projects. You can use the
`#[many_to_one]` attribute in order to do so. Just add:

```rust
#[ergol]
pub struct Project {
    #[id] pub id: i32,
    pub name: String,
    #[many_to_one(projects)] pub owner: User,
}
```

Once you have defined this struct, many more functions become available:

```rust
// Drop the user table if it exists
Project::drop_table().execute(&client).await.ok();
User::drop_table().execute(&client).await.ok();

// Create the user table
User::create_table().execute(&client).await?;
Project::create_table().execute(&client).await?;

// Create two users and save them into the database
let thomas: User = User::create("thomas", "pa$$word", 28).save(&client).await?;
User::create("nicolas", "pa$$word", 28).save(&client).await?;

// Create some projects for the user
let first_project = Project::create("My first project", &user).save(&client).await?;
Project::create("My second project", &user).save(&client).await?;

// You can easily find all projects from the user
let projects: Vec<Project> = thomas.projects(&client).await?;

// You can also find the owner of a project
let owner: User = projects[0].owner(&client).await?;
```

You can similarly have one-to-one relationship between a user and a project by
using the `#[one_to_one]` attribute:

```rust
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
// You can easily find a user's project
let project: Option<Project> = thomas.project(&client).await?;
```

Note that that way, a project has exactly one owner, but a user can have no
project.

## Many-to-many relationships

This macro also supports many-to-many relationships. In order to do so, you
need to use the `#[many_to_many]` attribute:

```rust
#[ergol]
pub struct Project {
    #[id] pub id: i32,
    pub name: String,
    #[many_to_many(projects)] pub authorized_users: User,
}
```

The same way, you will have plenty of functions that you will be able to use to
manage your objects:

```rust
// Find some users in the database
let thomas = User::get_by_username("thomas", &client).await?.unwrap();
let nicolas = User::get_by_username("nicolas", &client).await?.unwrap();

// Create a project
let first_project = Project::create("My first project").save(&client).await?;

// Both users can access this project
project.add_authorized_users(&thomas, &client).await?;
project.add_authorized_users(&nicolas, &client).await?;

// The second project can only be used by thomas
let second_project = Project::create("My second project").save(&client).await?;
project.add_authorized_users(&thomas, &client).await?;

// The third project can only be used by nicolas.
let third_project = Project::create("My third project").save(&client).await?;
project.add_authorized_users(&nicolas, &client).await?;

// You can easily retrieve all projects available for a certain user
let projects: Vec<Project> = thomas.projects(&client).await?;

// And you can easily retrieve all users that have access to a certain project
let users: Vec<User> = project.authorized_users(&client).await?;
```

## Limitations

For the moment, we still have plenty of limitations:

  - this crate only works with tokio-postgres
  - there is no support for migrations
  - the names of the structs you use in `#[ergol]` must be used previously, e.g.
    ```rust
    mod user {
        #[ergol]
        pub struct User {
            #[id] pub id: i32,
        }
    }

    #[ergol]
    pub struct Project {
        #[id] pub id: i32,
        #[many_to_one(projects)] pub owner: user::User, // this will not work
    }

    use user::User;

    #[ergol]
    pub struct Project {
        #[id] pub id: i32,
        #[many_to_one(projects)] pub owner: User, // this will work
    }
    ```

