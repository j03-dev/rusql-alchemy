use anyhow::Result;
use rusql_alchemy::prelude::*;
use sqlx::FromRow;

#[derive(FromRow, Clone, Debug, Default, Model)]
struct User {
    #[model(primary_key = true, auto = true)]
    id: Integer,

    #[model(size = 50, unique = true)]
    name: String,

    #[model(size = 255, unique = true)]
    email: Option<String>,

    #[model(size = 255)]
    password: String,

    #[model(default = false)]
    admin: Boolean,

    age: Integer,

    weight: Float,
}

#[derive(FromRow, Debug, Default, Model, Clone)]
struct Product {
    #[model(primary_key = true, auto = true)]
    id: Integer,

    #[model(size = 50)]
    name: String,

    price: Float,

    description: Option<Text>,

    #[model(default = false)]
    is_sel: Boolean,

    #[model(foreign_key = "User.id")]
    owner: Integer,

    #[model(default = "now")]
    at: DateTime,
}

#[tokio::main]
async fn main() -> Result<()> {
    let database = Database::new().await?;

    database.migrate().await?;

    let conn = database.conn;

    User {
        name: "johnDoe@gmail.com".to_string(),
        email: Some("21john@gmail.com".to_string()),
        password: "p455w0rd".to_string(),
        age: 18,
        weight: 60.0,
        ..Default::default()
    }
    .save(&conn)
    .await?;

    let users = User::all(&conn).await?;
    println!("0: {:#?}", users);

    User::create(
        kwargs!(
            name = "joe",
            email = "24nomeniavo@gmail.com",
            password = "strongpassword",
            age = 19,
            weight = 80.1
        ),
        &conn,
    )
    .await?;

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
    .await?;

    let products = Product::all(&conn).await;
    println!("3: {:#?}", products);

    let product = Product::get(kwargs!(is_sel == true), &conn).await;
    println!("4: {:#?}", product);

    let products = Product::all(&conn).await?;
    println!("5: {:#?}", products);
    products.delete(&conn).await?;

    let users = User::filter(kwargs!(age <= 18), &conn).await;
    println!("6: {:#?}", users);

    Ok(())
}
