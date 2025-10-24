use rusql_alchemy::prelude::*;
use serde::Deserialize;
use sqlx::FromRow;

#[derive(FromRow, Clone, Debug, Deserialize, Model)]
struct User {
    #[field(primary_key = true, auto = true)]
    id: Option<Integer>,

    #[field(size = 50, unique = true)]
    name: String,

    #[field(size = 255, unique = true)]
    email: Option<String>,

    #[field(size = 255)]
    password: String,

    age: Option<Integer>,

    #[field(default = false)]
    admin: Boolean,

    #[field(default = "user")]
    role: Option<String>,

    #[field(default = "now")]
    at: DateTime,
}

#[derive(FromRow, Debug, Deserialize, Model, Clone)]
struct Product {
    #[field(primary_key = true, auto = true)]
    id: Integer,

    #[field(size = 50)]
    name: String,

    price: Float,

    description: Option<Text>,

    #[field(default = false)]
    is_sel: Boolean,

    #[field(foreign_key = User.id)]
    owner: Integer,

    #[field(default = "now")]
    at: DateTime,
}

#[tokio::main]
async fn main() -> Result<(), rusql_alchemy::Error> {
    let database = Database::new().await?;

    database.migrate().await?;

    let conn = database.conn;

    User {
        name: "johnDoe@gmail.com".to_string(),
        email: Some("21john@gmail.com".to_string()),
        password: "p455w0rd".to_string(),
        ..Default::default()
    }
    .save(&conn)
    .await
    .ok();

    let users = User::all(&conn).await?;
    println!("0: {:#?}", users);

    User::create(
        kwargs!(
            name = "joe",
            email = "24nomeniavo@gmail.com",
            password = "strongpassword"
        ),
        &conn,
    )
    .await
    .ok();

    let users = User::all(&conn).await;
    println!("1: {:#?}", users);

    if let Some(mut user) = User::get(
        kwargs!(email == "24nomeniavo@gmail.com").and(kwargs!(password == "strongpassword")),
        &conn,
    )
    .await?
    {
        user.admin = True;
        user.update(&conn).await?;
    }
    let user = User::get(
        kwargs!(email == "24nomeniavo@gmail.com").and(kwargs!(password == "strongpassword")),
        &conn,
    )
    .await?;

    println!("2: {:#?}", user);

    Product::create(
        kwargs!(
            name = "tomato".to_string(),
            price = 1000.0,
            owner = user.clone().unwrap().id
        ),
        &conn,
    )
    .await
    .ok();

    let products = Product::all(&conn).await;
    println!("3: {:#?}", products);

    let product = Product::get(kwargs!(is_sel == false), &conn).await;
    println!("4: {:#?}", product);

    let products = Product::all(&conn).await?;
    println!("5: {:#?}", products);
    products.delete(&conn).await?;

    let user_count = User::count(&conn).await;
    println!("6: {:#?}", user_count);

    Ok(())
}
