#[macro_use]
extern crate rocket;

use anyhow::{Context, Result};
use rocket::serde::json::{json, Value};
use rocket::State;
use rusql_alchemy::prelude::*;
use serde::Serialize;

#[derive(Clone)]
struct AppState {
    conn: Connection,
}

#[derive(Model, FromRow, Clone, Serialize)]
struct User_ {
    #[model(primary_key = true)]
    id: Serial,

    #[model(unique = true, size = 50)]
    username: String,
}

#[get("/users")]
async fn list_user(app_state: &State<AppState>) -> Value {
    let conn = app_state.conn.clone();
    let users = User_::all(&conn).await.unwrap();
    json!(users)
}

#[main]
async fn main() -> Result<()> {
    let conn = Database::new().await?.conn;

    for model in inventory::iter::<MigrationRegistrar> {
        (model.migrate_fn)(conn.clone()).await?;
    }

    rocket::build()
        .mount("/", routes![list_user])
        .manage(AppState { conn })
        .launch()
        .await
        .context("failed to launch rocket instance")?;

    Ok(())
}
