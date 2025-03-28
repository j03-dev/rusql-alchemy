use anyhow::Result;
use rusql_alchemy::prelude::*;
use sqlx::FromRow;

#[derive(FromRow, Clone, Debug, Model)]
struct User_ {
    #[field(primary_key = true)]
    id: Serial, // in postgresql, serial is auto increment

    #[field(size = 50, unique = true)]
    name: String,

    #[field(size = 255, unique = true)]
    email: Option<String>,

    #[field(size = 255)]
    password: String,

    #[field(default = "user")]
    role: String,

    #[field]
    age: Integer,

    #[field]
    weight: Float,
}

#[derive(FromRow, Debug, Model, Clone)]
struct Product {
    #[field(primary_key = true)]
    id: Serial, // in postgresql, serial is auto increment

    #[field(size = 50)]
    name: String,

    #[field]
    price: Float,

    #[field]
    description: Option<Text>,

    #[field(default = true)]
    is_sel: Boolean,

    #[field(foreign_key = User_.id)]
    owner: Integer,

    #[field(default = "now")]
    at: DateTime,
}

#[tokio::main]
async fn main() -> Result<()> {
    let database = Database::new().await?;

    database.migrate().await?;

    let conn = database.conn;

    User_ {
        name: "johnDoe@gmail.com".to_string(),
        email: Some("21john@gmail.com".to_string()),
        password: "p455w0rd".to_string(),
        age: 18,
        weight: 60.0,
        ..Default::default()
    }
    .save(&conn)
    .await?;

    let users = User_::all(&conn).await;
    println!("{:#?}", users);

    User_::create(
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

    let users = User_::all(&conn).await;
    println!("1: {:#?}", users);

    if let Some(mut user) = User_::get(
        kwargs!(email == "24nomeniavo@gmail.com").and(kwargs!(password == "strongpassword")),
        &conn,
    )
    .await?
    {
        user.role = "admin".into();
        user.update(&conn).await?;
    }
    let user = User_::get(
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

    let users = User_::filter(kwargs!(age <= 18), &conn).await;
    println!("6: {:#?}", users);

    Ok(())
}
