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
    #[field(primary_key = true)]
    id: Serial,

    #[field(unique = true, size = 50)]
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
    let database = Database::new().await?;

    database.migrate().await?;

    rocket::build()
        .mount("/", routes![list_user])
        .manage(AppState {
            conn: database.conn.clone(),
        })
        .launch()
        .await
        .context("failed to launch rocket instance")?;

    Ok(())
}
