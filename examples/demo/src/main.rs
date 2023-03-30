use ergol::prelude::*;
use ergol::tokio;
use ergol::tokio_postgres::{Error, NoTls};

#[ergol]
pub struct User {
    #[id]
    pub id: i32,

    #[unique]
    pub username: String,

    #[unique]
    pub email: String,

    pub age: i32,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let (client, connection) =
        ergol::connect("host=localhost user=ergol password=ergol", NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    // Try to delete the database
    User::drop_table().execute(&client).await.ok();

    // Create the tables
    User::create_table().execute(&client).await?;

    // Create users
    User::create("graydon", "graydon@example.com", 28)
        .save(&client)
        .await?;
    User::create("evan", "evan@example.com", 4)
        .save(&client)
        .await?;
    User::create("nico", "nico@example.com", 28)
        .save(&client)
        .await?;

    let thomas = User::create("tforgione", "thomas@forgione.fr", 28)
        .save(&client)
        .await?;

    println!("{:?}", thomas);

    // Change user attributs in single request
    let thomas = UserChangeSet::from_id(4)
        .email("thomas@polymny.studio")
        .age(30)
        .execute(&client)
        .await?;

    println!("{:?}", thomas);

    Ok(())
}
