use rust_alchemy::prelude::*;
use serde::Deserialize;

#[derive(Deserialize, Debug, Default, rust_alchemy_macro::Model)]
struct User {
    #[model(primary_key = true, auto = true, null = false)]
    id: i32,
    #[model(size = 50, null = false)]
    name: String,
    #[model(size = 255, unique, null = true)]
    email: String,
    #[model(size = 255, null = false)]
    password: String,
    bio: Text,
    #[model(default = "now")]
    birth: Date,
}

#[derive(Deserialize, Debug, Default, rust_alchemy_macro::Model)]
struct Product {
    #[model(primary_key = true, auto = true, null = false)]
    id: i32,
    #[model(size = 50, null = false)]
    name: String,
    price: Float,
    #[model(null = false, foreign_key = "User.id")]
    owner: i32,
}

#[tokio::main]
async fn main() {
    println!("{}", User::schema());
    println!("{}", Product::schema());

    let conn = config::db::Database::new().await.conn;

    migrate!([User, Product], &conn);

    let user = User {
        name: "John Doe".to_string(),
        email: "johndoe@gmailcom".to_string(),
        password: "password".to_string(),
        ..Default::default()
    };

    user.save(&conn).await;

    User::create(
        kwargs!(
            name = "joe",
            email = "24nomeniavo@gmail.com",
            password = "password"
        ),
        &conn,
    )
    .await;

    let user = User::get(kwargs!(name = "John Doe"), &conn).await;
    User {
        email: "21johndoe@gmail.com".to_string(),
        ..user
    }
    .update(&conn)
    .await;

    let users = User::filter(kwargs!(name = "John Doe", name = "joe").or(), &conn).await;
    println!("{:#?}", users);
}
