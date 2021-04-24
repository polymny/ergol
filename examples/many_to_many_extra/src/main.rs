use ergol::prelude::*;
use ergol::tokio;
use ergol::tokio_postgres::{Error, NoTls};

#[rustfmt::skip]
#[ergol]
pub struct User {
    #[id] pub id: i32,
    #[unique] pub username: String,
}

#[derive(PgEnum, Debug)]
pub enum Role {
    Admin,
    Write,
    Read,
}

#[rustfmt::skip]
#[ergol]
pub struct Project {
    #[id] pub id: i32,
    pub name: String,
    #[many_to_many(projects, Role)] pub users: User,
}

#[rustfmt::skip]
#[tokio::main]
async fn main() -> Result<(), Error> {
    let (client, connection) = ergol::connect(
        "host=localhost user=ergol password=ergol",
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
    Role::drop_type().execute(&client).await.ok();

    // Create the tables
    Role::create_type().execute(&client).await?;
    User::create_table().execute(&client).await?;
    Project::create_table().execute(&client).await?;

    // Create users
    User::create("graydon").save(&client).await?;
    User::create("evan").save(&client).await?;
    User::create("nico").save(&client).await?;
    User::create("tforgione").save(&client).await?;

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

    tforgione.update_role(&project, Role::Admin, &client).await?;

    for (project, role) in tforgione.projects(&client).await? {
        println!("{} has {:?} rights on project {:?}", tforgione.username, role, project.name);
    }

    project.update_role(&tforgione, Role::Write, &client).await?;

    for (project, role) in tforgione.projects(&client).await? {
        println!("{} has {:?} rights on project {:?}", tforgione.username, role, project.name);
    }

    Ok(())
}
