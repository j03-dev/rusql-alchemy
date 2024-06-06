use rusql_alchemy::prelude::*;
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug, Default, Model)]
struct User {
    #[model(primary_key = true, auto = true, null = false)]
    id: i32,
    #[model(size = 50, unique = true, null = false)]
    name: String,
    #[model(size = 255, unique = true, null = true)]
    email: String,
    #[model(size = 255, null = false)]
    password: String,
    birth: Date,
    #[model(default = "user")]
    role: String,
}

#[derive(Deserialize, Debug, Default, Model)]
struct Product {
    #[model(primary_key = true, auto = true, null = false)]
    id: i32,
    #[model(size = 50, null = false)]
    name: String,
    price: Float,
    description: Text,
    #[model(default = "now")]
    at: DateTime,
    #[model(null = false, foreign_key = "User.id")]
    owner: i32,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    println!("{}", User::SCHEMA);
    println!("{}", Product::SCHEMA);

    let conn = config::db::Database::new().await.conn;

    migrate!([User, Product], &conn);

    User {
        name: "johnDoe@gmail.com".to_string(),
        email: "21john@gmail.com".to_string(),
        password: "p455w0rd".to_string(),
        birth: "01-01-1999".to_string(),
        ..Default::default()
    }
    .save(&conn)
    .await;

    User::create(
        kwargs!(
            name = "joe",
            email = "24nomeniavo@gmail.com",
            password = "strongpassword",
            birth = "24-03-2001"
        ),
        &conn,
    )
    .await;

    let users = User::all(&conn).await;
    println!("1: {:#?}", users);

    let user = User::get(
        kwargs!(email = "24nomeniavo@gmail.com", password = "strongpassword"),
        &conn,
    )
    .await;
    println!("2: {:#?}", user);

    if let Some(user) = user {
        let update = User {
            role: "admin".to_string(),
            ..user.clone()
        }
        .update(&conn)
        .await;
        println!("3: {update}");
    }

    if let Some(user) = User::get(kwargs!(id = 1), &conn).await {
        user.delete(&conn).await;
    }

    let users = User::all(&conn).await;
    println!("4: {:#?}", users);
}
