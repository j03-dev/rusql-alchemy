use rusql_alchemy::prelude::*;
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug, Default, Model)]
struct User {
    #[model(primary_key = true, auto = true, null = false)]
    id: Integer,
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

#[derive(Deserialize, Debug, Default, Model, Clone)]
struct Product {
    #[model(primary_key = true, auto = true, null = false)]
    id: Integer,
    #[model(size = 50, null = false)]
    name: String,
    price: Float,
    description: Text,
    #[model(default = "now")]
    at: DateTime,
    #[model(default = true)]
    is_sel: bool,
    #[model(null = false, foreign_key = "User.id")]
    owner: Integer,
}

#[tokio::main]
async fn main() {
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

    Product {
        name: "tomato".to_string(),
        price: 1000.0,
        description: "".to_string(),
        owner: user.clone().unwrap().id,
        ..Default::default()
    }
    .save(&conn)
    .await;

    let products = Product::all(&conn).await;
    println!("3: {:#?}", products);

    let users = User::all(&conn).await;
    println!("4: {:#?}", users);

    let product = Product::get(kwargs!(is_sel = true), &conn).await;
    println!("5: {:#?}", product);

    let user = User::get(kwargs!(owner__product__is_sel = true), &conn).await;
    println!("6: {:#?}", user);
}
