# ergol

[![CI](https://github.com/polymny/ergol/workflows/build/badge.svg?branch=master&event=push)](https://github.com/polymny/ergol/actions?query=workflow%3Abuild)

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
let thomas: User = User::create("thomas", "pa$$w0rd", 28).save(&client).await?;
User::create("nicolas", "pa$$w0rd", 28).save(&client).await?;

// Create some projects for the user
let project: Project = Project::create("My first project", &thomas).save(&client).await?;
Project::create("My second project", &thomas).save(&client).await?;

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


This macro also supports many-to-many relationships. In order to do so, you
need to use the `#[many_to_many]` attribute:

```rust
#[ergol]
pub struct Project {
    #[id] pub id: i32,
    pub name: String,
    #[many_to_many(visible_projects)] pub authorized_users: User,
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

// Thomas can access this project
first_project.add_authorized_user(&thomas, &client).await?;

// The other way round
nicolas.add_visible_project(&first_project, &client).await?;

// The second project can only be used by thomas
let second_project = Project::create("My second project").save(&client).await?;
thomas.add_visible_project(&second_project, &client).await?;

// The third project can only be used by nicolas.
let third_project = Project::create("My third project").save(&client).await?;
third_project.add_authorized_user(&nicolas, &client).await?;

// You can easily retrieve all projects available for a certain user
let projects: Vec<Project> = thomas.visible_projects(&client).await?;

// And you can easily retrieve all users that have access to a certain project
let users: Vec<User> = first_project.authorized_users(&client).await?;

// You can easily remove a user from a project
let _: bool = first_project.remove_authorized_user(&thomas, &client).await?;

// Or vice-versa
let _: bool = nicolas.remove_visible_project(&first_project, &client).await?;

// The remove functions return true if they successfully removed something.
```


It is possible to insert some extra information in a many to many relationship. The following
exemple gives roles for the users for projects.

```rust
#[ergol]
pub struct User {
    #[id] pub id: i32,
    #[unique] pub username: String,
    pub password: String,
}

#[derive(PgEnum, Debug)]
pub enum Role {
   Admin,
   Write,
   Read,
}

#[ergol]
pub struct Project {
    #[id] pub id: i32,
    pub name: String,
    #[many_to_many(projects, Role)] pub users: User,
}
```

With these structures, the signature of generated methods change to take a role as argument,
and to return tuples of (User, Role) or (Project, Role).

```rust
let tforgione = User::get_by_username("tforgione", &client).await?.unwrap();
let graydon = User::get_by_username("graydon", &client).await?.unwrap();
let project = Project::create("My first project").save(&client).await?;
project.add_user(&tforgione, Role::Admin, &client).await?;
graydon.add_project(&project, Role::Read, &client).await?;

for (user, role) in project.users(&client).await? {
    println!("{} has {:?} rights on project {:?}", user.username, role, project.name);
}

let project = Project::create("My second project").save(&client).await?;
project.add_user(&tforgione, Role::Admin, &client).await?;

let project = Project::create("My third project").save(&client).await?;
project.add_user(&graydon, Role::Admin, &client).await?;
project.add_user(&tforgione, Role::Read, &client).await?;

for (project, role) in tforgione.projects(&client).await? {
    println!("{} has {:?} rights on project {:?}", tforgione.username, role, project.name);
}
```


For the moment, we still have plenty of limitations:

  - this crate only works with tokio-postgres
  - there is no support for migrations
  - the names of the structs you use in `#[ergol]` must be used previously, e.g.
    ```rust,ignore
    mod user {
        use ergol::prelude::*;

        #[ergol]
        pub struct User {
            #[id] pub id: i32,
        }
    }

    use ergol::prelude::*;
    #[ergol]
    pub struct Project {
        #[id] pub id: i32,
        #[many_to_one(projects)] pub owner: user::User, // this will not work
    }
    ```

    ```rust
    mod user {
        use ergol::prelude::*;
        #[ergol]
        pub struct User {
            #[id] pub id: i32,
        }
    }
    use user::User;

    use ergol::prelude::*;
    #[ergol]
    pub struct Project {
        #[id] pub id: i32,
        #[many_to_one(projects)] pub owner: User, // this will work
    }
    ```
