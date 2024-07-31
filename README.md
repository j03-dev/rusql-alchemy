# Rusql Alchemy

## Rusql Alchemy is Django ORM like library for Rust

### Why ?

Just for fun! XD

## Sqlite

### Setup `.env` file
```bash
DATABASE_URL=sqlite://<dabasase.db>
```
### Setup `Cargo.toml`
```toml
[dependencies.rusql-alchemy]
git = "https://github.com/j03-dev/rusql-alchemy"
branch= "main"
features = ["sqlite"] # the default features is sqlite
```
### Model
```rust
use rusql_alchemy::prelude::*;

#[derive(Debug, Model, FromRow)]
struct User {
    #[model(primary_key=true, auto=true, null=false)]
    id: Integer,
    #[model(unique=true, null=false)]
    name: String,
    #[model(null=false)]
    age: Integer,
    #[model(default="user")]
    role: String
}
```
## Postgres

### Setup `.env` file

``` bash
DATABASE_URL=postgres://<user>:<password>@<hostname>/<dbname>
```

### Setup `Cargo.toml`
```toml
[dependencies.rusql-alchemy]
git = "https://github.com/j03-dev/rusql-alchemy"
branch="main"
default-features = false
features = ["postgres"]
```
### Model: In postgres primary key should be `Serial` type
```rust
use rusql_alchemy::prelude::*;

#[derive(Debug, Model, FromRow)]
struct User_ {
    #[model(primary_key=true)]
    id: Serial,
    #[model(unique=true, null=false)]
    name: String,
    #[model(null=false)]
    age: Integer,
    #[model(default="user")]
    role: String
}
```

## Migrate

```rust
use rusql_alchemy::prelude::*;

#[tokio::main]
async fn main() {
    let conn = Database::new().await.conn;
    migrate([Use], &conn);
}
```
## Query

### Insert
```rust
use rusql_alchemy::prelude::*;

#[tokio::main]
async fn main() {
    let conn = Database::new().await.conn;

    User_ {
        name: "johnDoe@gmail.com".to_string(),
        email: "21john@gmail.com".to_string(),
        password: "p455w0rd".to_string(),
        age: 18,
        weight: 60.0,
        ..Default::default()
    }
        .save(&conn)
        .await;

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
    .await;
}
```
### Select
```rust
use rusql_alchemy::prelude::*;

#[tokio::main]
async fn main() {
    let conn = config::db::Database::new().await.conn;

    let users = User_::all(&conn).await;
    println!("{:#?}", users);

    let user = User_::get(
        kwargs!(email == "24nomeniavo@gmail.com").and(kwargs!(password == "strongpassword")),
        &conn,
    ).await;
    println!("{:#?}", user);

    let users = User_::filter(kwargs!(age <= 18), &conn).await;
    println!("{:#?}", users);
}
```
### Update
```rust
use rusql_alchemy::prelude::*;

#[tokio::main]
async fn main() {
    let conn = Database::new().await.conn;

    if let Some(mut user) = User_::get(
        kwargs!(email == "24nomeniavo@gmail.com").and(kwargs!(password == "strongpassword")),
        &conn,
    )
    .await
    {
        user.role = "admin".into();
        user.update(&conn).await;
    }
}
```
### Delete
```rust
use rusql_alchemy::prelude::*;

#[tokio::main]
async fn main() {
    let conn = Database::new().await.conn;

    if let Some(user) = User_::get(kwargs!(role == "admin"), &conn).await {
        user.delete(&conn).await; // delete one
    }
    
    let users = User_::all(&conn).await;
    users.delete(&conn).await; // delete all
}
```
