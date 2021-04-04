#[macro_use]
extern crate rocket;

use rocket::fairing::AdHoc;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::{Rocket, State};

use ergol::deadpool::managed::Object;
use ergol::prelude::*;

/// A wrapper for a database connection extrated from a pool.
pub struct Db(Object<Ergol, ergol::tokio_postgres::Error>);

impl Db {
    /// Extracts a database from a pool.
    pub async fn from_pool(pool: ergol::Pool) -> Db {
        Db(pool.get().await.unwrap())
    }
}

// This allows to pass directly Db instead of Ergol to the ergol's functions.
impl std::ops::Deref for Db {
    type Target = Object<Ergol, ergol::tokio_postgres::Error>;
    fn deref(&self) -> &Self::Target {
        &*&self.0
    }
}

// This allows to use Db in routes parameters.
#[rocket::async_trait]
impl<'r> FromRequest<'r> for Db {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let pool = request.guard::<State<ergol::Pool>>().await.unwrap();
        let db = pool.get().await.unwrap();
        Outcome::Success(Db(db))
    }
}

/// Creates the database fairing to be able to use the database in the routes.
async fn db_fairing(rocket: Rocket) -> Result<Rocket, Rocket> {
    let pool = ergol::pool("host=localhost user=ergol password=ergol", 32);
    Ok(rocket.manage(pool))
}

#[ergol]
pub struct Item {
    #[id]
    id: i32,
    name: String,
    count: i32,
}

#[get("/add-item/<name>/<count>")]
async fn add_item(name: String, count: i32, db: Db) -> String {
    Item::create(name, count).save(&db).await.unwrap();
    "Item added".into()
}

#[get("/")]
async fn list_items(db: Db) -> String {
    let items = Item::select()
        .execute(&db)
        .await
        .unwrap()
        .into_iter()
        .map(|x| format!("  - {}: {}", x.name, x.count))
        .collect::<Vec<_>>()
        .join("\n");

    format!("{}\n{}", "List of items:", items)
}

#[launch]
async fn rocket() -> rocket::Rocket {
    // Setup rocket with its database connections pool.
    let rocket = rocket::ignite()
        .attach(AdHoc::on_attach("Database", db_fairing))
        .mount("/", routes![list_items, add_item]);

    // Get the pool from rocket.
    let pool = rocket.state::<ergol::Pool>().unwrap();

    {
        // Reset the Db at startup (you may not want to do this, but it's cool for an example.
        let db = Db::from_pool(pool.clone()).await;
        Item::drop_table().execute(&db).await.ok();
        Item::create_table().execute(&db).await.unwrap();
    }

    // Return the rocket instance to start the server.
    rocket
}
