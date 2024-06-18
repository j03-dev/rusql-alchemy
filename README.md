# Rusql Alchemy

## Rusql Alchemy is ORM for `Turso` Database

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
branch="main"
features = ["sqlite"]
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
features = ["postgres"]
```
### Model: In postgres primary key should be `Serial` type
```rust
use rusql_alchemy::prelude::*;

#[derive(Debug, Model, FromRow)]
struct User {
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
    let conn = config::db::Database::new().await.conn;
    migrate([Use], &conn);
}
```
## Query

### Insert
```rust
#[tokio::main]
async fn main() {
    let conn = config::db::Database::new().await.conn;

    UserTest {
        name: "johnDoe@gmail.com".to_string(),
        email: "21john@gmail.com".to_string(),
        password: "p455w0rd".to_string(),
        age: 18,
        weight: 60.0,
        ..Default::default()
    }
        .save(&conn)
        .await;

    let users = UserTest::all(&conn).await;
    println!("{:#?}", users);

    UserTest::create(
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
#[tokio::main]
async fn main() {
    let conn = config::db::Database::new().await.conn;

    let users = UserTest::all(&conn).await;
    println!("{:#?}", users);

    let user = UserTest::get(
        kwargs!(email = "24nomeniavo@gmail.com", password = "strongpassword"),
        &conn,
    ).await;
    println!("{:#?}", user);

    let users = UserTest::filter(kwargs!(role = "user"), &conn).await;
    println!("{:#?}", users);
}
```
### Update
```rust
#[tokio::main]
async fn main() {
    let conn = config::db::Database::new().await.conn;
    if let Some(user) = UserTest::get(
        kwargs!(email = "24nomeniavo@gmail.com", password = "strongpassword"),
        &conn,
    )
    .await
    {
        UserTest {
            role: "admin".into(),
            ..user
        }
        .update(&conn)
        .await;
    }
}
```
### Delete
```rust
#[tokio::main]
async fn main() {
    let conn = config::db::Database::new().await.conn;

    if let Some(user) = UserTest::get(kwargs!(role = "admin"), &conn).await {
        user.delete(&conn).await; // delete one
    }
    
    let users = UserTest::all(&conn).await;
    user.delete(&conn).await; // delete all
}
```