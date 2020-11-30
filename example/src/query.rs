use ergol::prelude::*;
use ergol::tokio;
use ergol::tokio_postgres::{Error, NoTls};

#[rustfmt::skip]
#[ergol]
pub struct User {
    #[id] pub id: i32,
    #[unique] pub username: String,
    #[unique] pub email: String,
    pub age: i32,
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
        "host=localhost user=orm",
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

    // Create the tables
    User::create_table().execute(&client).await?;
    Project::create_table().execute(&client).await?;

    // Create users
    User::create("graydon", "graydon@example.com", 28).save(&client).await?;
    User::create("evan", "evan@example.com", 4).save(&client).await?;
    User::create("nico", "nico@example.com", 28).save(&client).await?;
    User::create("tforgione", "thomas@forgione.fr", 28).save(&client).await?;

    let tforgione = User::get_by_username("tforgione", &client).await?.unwrap();
    User::get_by_username("graydon", &client).await?.unwrap();

    Project::create("My first project", &tforgione).save(&client).await?;
    Project::create("My second project", &tforgione).save(&client).await?;
    Project::create("My third project", &tforgione).save(&client).await?;

    // Select all users
    let users = User::select().filter(user::age::eq(28)).execute(&client).await?;
    for user in users {
        println!("{} is {} years old", user.username, user.age);
    }

    Ok(())
}
