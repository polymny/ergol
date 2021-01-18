use ergol::prelude::*;
use ergol::tokio;
use ergol::tokio_postgres::types::Json;
use ergol::tokio_postgres::{Error, NoTls};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Data {
    x: f64,
    y: f64,
}

impl Data {
    pub fn new() -> Json<Data> {
        Json(Data { x: 0.0, y: 0.0 })
    }
}

#[rustfmt::skip]
#[ergol]
pub struct User {

    /// The comment of the id.
    #[id] pub id: i32,
    #[unique] pub username: String,
    #[unique] pub email: String,
    pub age: Option<i32>,
    /// The comment of the gender.
    pub gender: Gender,
    pub json: Json<Data>,
}

#[derive(PgEnum, Copy, Clone, Debug)]
pub enum Gender {
    Male,
    Female,
    Other,
}

#[rustfmt::skip]
#[ergol]
pub struct Project {
    #[id] pub id: i32,
    pub name: String,
    #[many_to_one(projects)] pub owner: User,
}

#[rustfmt::skip]
#[tokio::main]
async fn main() -> Result<(), Error> {
    let (client, connection) = ergol::tokio_postgres::connect(
        "host=localhost user=ergol",
        NoTls,
    )
    .await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    // Try to delete the database
    User::drop_table().execute(&client).await.ok();
    Project::drop_table().execute(&client).await.ok();
    Gender::drop_type().execute(&client).await.ok();

    // Create the tables
    Gender::create_type().execute(&client).await?;
    User::create_table().execute(&client).await?;
    Project::create_table().execute(&client).await?;

    // Create users
    User::create("graydon", "graydon@example.com", None, Gender::Male, Data::new()).save(&client).await?;
    User::create("evan", "evan@example.com", Some(4), Gender::Male, Data::new()).save(&client).await?;
    User::create("nico", "nico@example.com", None, Gender::Male, Data::new()).save(&client).await?;
    User::create("tforgione", "thomas@forgione.fr", Some(28), Gender::Male, Data::new()).save(&client).await?;

    let tforgione = User::get_by_username("tforgione", &client).await?.unwrap();
    let graydon = User::get_by_username("graydon", &client).await?.unwrap();

    Project::create("My first project", &tforgione).save(&client).await?;
    Project::create("My second project", &tforgione).save(&client).await?;
    Project::create("My third project", &tforgione).save(&client).await?;

    // Select all users
    let mut users = User::select().execute(&client).await?;

    // Update the age
    println!("Before update");
    for user in &mut users {
        println!("{} {:?} {:?}", user.username, user.age, user.gender);
        if let Some(age) = user.age.as_mut() {
            *age += 1;
        }
        user.gender = Gender::Other;
        user.save(&client).await?;
    }

    // Re-select to verify the update
    println!("\nAfter update");
    let users = User::select().execute(&client).await?;
    for user in users {
        println!("{} {:?} {:?}", user.username, user.age, user.gender);
    }

    // Select all projects
    println!("\nProjects");
    let mut projects = Project::select().execute(&client).await?;
    for project in &mut projects {
        let owner = project.owner(&client).await?;
        project.owner = (&graydon).into();
        project.save(&client).await?;
        println!("Project \"{}\" owned by \"{}\"", project.name, owner.username);
    }

    // Exploit the many to one relation
    let projects = tforgione.projects(&client).await?;
    println!("\n{}'s projects ({} projects):", tforgione.username, projects.len());
    for project in projects {
        println!("  - {}", project.name);
    }

    let projects = graydon.projects(&client).await?;
    println!("\n{}'s projects ({} projects):", graydon.username, projects.len());
    for project in projects {
        println!("  - {}", project.name);
    }

    Ok(())
}
