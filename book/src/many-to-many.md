# Many-to-many relationships

Ergol also supports many-to-many relationships. In order to do so, you need to
use the `#[many_to_many]` attribute:

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
# #[ergol]
# pub struct Project {
#     #[id] pub id: i32,
#     pub name: String,
#     #[many_to_many(visible_projects)] pub authorized_users: User,
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
# Project::drop_table().execute(&db).await.ok();
# User::drop_table().execute(&db).await.ok();
# User::create_table().execute(&db).await?;
# Project::create_table().execute(&db).await?;
# User::create("thomas", "pa$$w0rd", 28).save(&db).await?;
# User::create("nicolas", "pa$$w0rd", 28).save(&db).await?;
// Find some users in the database
let thomas = User::get_by_username("thomas", &db).await?.unwrap();
let nicolas = User::get_by_username("nicolas", &db).await?.unwrap();

// Create a project
let first_project = Project::create("My first project").save(&db).await?;

// Thomas can access this project
first_project.add_authorized_user(&thomas, &db).await?;

// The other way round
nicolas.add_visible_project(&first_project, &db).await?;

// The second project can only be used by thomas
let second_project = Project::create("My second project").save(&db).await?;
thomas.add_visible_project(&second_project, &db).await?;

// The third project can only be used by nicolas.
let third_project = Project::create("My third project").save(&db).await?;
third_project.add_authorized_user(&nicolas, &db).await?;

// You can easily retrieve all projects available for a certain user
let projects: Vec<Project> = thomas.visible_projects(&db).await?;

// And you can easily retrieve all users that have access to a certain project
let users: Vec<User> = first_project.authorized_users(&db).await?;

// You can easily remove a user from a project
let _: bool = first_project.remove_authorized_user(&thomas, &db).await?;

// Or vice-versa
let _: bool = nicolas.remove_visible_project(&first_project, &db).await?;

// The remove functions return true if they successfully removed something.
# Ok(())
# }
```

### Extra information in a many-to-many relationship

It is possible to insert some extra information in a many to many relationship. The following
exemple gives roles for the users for projects.

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
# #[derive(PgEnum, Debug)]
# pub enum Role {
#    Admin,
#    Write,
#    Read,
# }
# #[ergol]
# pub struct Project {
#     #[id] pub id: i32,
#     pub name: String,
#     #[many_to_many(projects, Role)] pub users: User,
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
# Project::drop_table().execute(&db).await.ok();
# User::drop_table().execute(&db).await.ok();
# Role::drop_type().execute(&db).await.ok();
# Role::create_type().execute(&db).await?;
# User::create_table().execute(&db).await?;
# Project::create_table().execute(&db).await?;
# User::create("thomas", "pa$$w0rd", 28).save(&db).await?;
# User::create("nicolas", "pa$$w0rd", 28).save(&db).await?;
let thomas = User::get_by_username("thomas", &db).await?.unwrap();
let nicolas = User::get_by_username("nicolas", &db).await?.unwrap();
let project = Project::create("My first project").save(&db).await?;
project.add_user(&thomas, Role::Admin, &db).await?;
nicolas.add_project(&project, Role::Read, &db).await?;

for (user, role) in project.users(&db).await? {
    println!("{} has {:?} rights on project {:?}", user.username, role, project.name);
}

let project = Project::create("My second project").save(&db).await?;
project.add_user(&thomas, Role::Admin, &db).await?;

let project = Project::create("My third project").save(&db).await?;
project.add_user(&nicolas, Role::Admin, &db).await?;
project.add_user(&thomas, Role::Read, &db).await?;

for (project, role) in thomas.projects(&db).await? {
    println!("{} has {:?} rights on project {:?}", thomas.username, role, project.name);
}
# Ok(())
# }
```
