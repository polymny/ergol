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
    #[id] pub id: i32,
    #[unique] pub username: String,
    #[unique] pub email: String,
    pub age: Option<i32>,
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
    #[one_to_one(project)] pub owner: User,
}

#[rustfmt::skip]
#[tokio::main]
async fn main() -> Result<(), Error> {
    let (mut client, connection) = ergol::connect(
        "host=localhost user=ergol password=ergol",
        NoTls,
    )
    .await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let mut db = client.transaction().await.unwrap();

    // Try to delete the database
    // User::drop_table().execute(&db).await.ok();
    // Project::drop_table().execute(&db).await.ok();
    // Gender::drop_type().execute(&db).await.ok();

    // // Create the tables
    // Gender::create_type().execute(&db).await?;
    // User::create_table().execute(&db).await?;
    // Project::create_table().execute(&db).await?;

    // Create users
    User::create("graydon", "graydon@example.com", None, Gender::Male, Data::new()).save(&mut db).await?;
    // User::create("evan", "evan@example.com", Some(4), Gender::Male, Data::new()).save(&db).await?;
    // User::create("nico", "nico@example.com", None, Gender::Male, Data::new()).save(&db).await?;
    // User::create("tforgione", "thomas@forgione.fr", Some(28), Gender::Male, Data::new()).save(&db).await?;

    // let tforgione = User::get_by_username("tforgione", &db).await?.unwrap();
    // let graydon = User::get_by_username("graydon", &db).await?.unwrap();

    // Project::create("My first project", &tforgione).save(&db).await?;

    // // Select all users
    // let mut users = User::select().execute(&db).await?;

    // // Update the age
    // println!("Before update");
    // for user in &mut users {
    //     println!("{} {:?} {:?}", user.username, user.age, user.gender);
    //     if let Some(age) = user.age.as_mut() {
    //         *age += 1;
    //     }
    //     user.gender = Gender::Other;
    //     user.save(&db).await?;
    // }

    // // Re-select to verify the update
    // println!("\nAfter update");
    // let users = User::select().execute(&db).await?;
    // for user in users {
    //     println!("{} {:?} {:?}", user.username, user.age, user.gender);
    // }

    // // Select all projects
    // println!("\nProjects");
    // let mut projects = Project::select().execute(&db).await?;
    // for project in &mut projects {
    //     let owner = project.owner(&db).await?;
    //     project.owner = (&graydon).into();
    //     project.save(&db).await?;
    //     println!("Project \"{}\" owned by \"{}\"", project.name, owner.username);
    // }

    // // Select all projects
    // println!("\nProjects");
    // let projects = Project::select().execute(&db).await?;
    // for project in projects {
    //     let owner = project.owner(&db).await?;
    //     println!("Project \"{}\" owned by \"{}\"", project.name, owner.username);
    // }

    // // Exploit the one to one relation
    // println!("\nProjects");
    // let project = tforgione.project(&db).await?;
    // println!("{}'s project is {:?}", tforgione.username, project.map(|x| x.name));

    // let project = graydon.project(&db).await?;
    // println!("{}'s project is {:?}", graydon.username, project.map(|x| x.name));

    Ok(())
}
