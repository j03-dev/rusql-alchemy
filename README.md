# RustAlchemy

## RustAlchemy is ORM for `Turso` Database

### Why ?

Just for fun! XD

## Example

```rust
use rust_alchemy::prelude::*;
use rust_alchemy_macro::Model;
use serde::Deserialize;

#[derive(Deserialize, Debug, Default, Model)]
struct User {
    #[model(primary_key = true, auto = true, null = false)]
    id: i32,
    #[model(size = 50, null = false)]
    name: String,
    #[model(size = 255, unique, null = true)]
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
    #[model(null = false, foreign_key = "User.id")]
    owner: i32,
    description: Text,
    #[model(default = "now")]
    at: DateTime,
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
		birth: "01-01-1990".to_string(),
        ..Default::default()
    };

    user.save(&conn).await;

    User::create(
        kwargs!(
            name = "joe",
            email = "24nomeniavo@gmail.com",
            password = "password",
			birth = "24-03-2001"
        ),
        &conn,
    )
    .await;

    let user = User::get(kwargs!(name = "joe"), &conn).await;
    User {
		role: "admin".to_string(),
        ..user
    }
    .update(&conn)
    .await;

    let users = User::filter(kwargs!(name = "John Doe", name = "joe").or(), &conn).await;
    println!("{:#?}", users);
}
```
