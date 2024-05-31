use rust_alchemy::db::models::Model;

use rust_alchemy::args;
use rust_alchemy_macro::Model;
use serde::Deserialize;

#[derive(Model, Deserialize)]
struct User {
    name: String,
    email: String,
    password: String,
}

#[tokio::main]
async fn main() {
    let user = User {
        name: "John Doe".to_string(),
        email: "johndoe@gmailcom".to_string(),
        password: "password".to_string(),
    };

    user.save().await;
    User::create(args!(
        name = "joe",
        email = "24nomeniavo@gmail.com",
        password = "password"
    ))
    .await;
    User::get(args!(name = "John Doe")).await;
    User::filter(args!(name = "John Doe", name = "joe").or()).await;
}
