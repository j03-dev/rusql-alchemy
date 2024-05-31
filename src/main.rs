use rust_alchemy::db::models::Model;

use rust_alchemy::kwargs;
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
    User::create(kwargs!(
        name = "joe",
        email = "24nomeniavo@gmail.com",
        password = "password"
    ))
    .await;
    User::get(kwargs!(name = "John Doe")).await;
    User::filter(kwargs!(name = "John Doe", name = "joe").or()).await;
}
