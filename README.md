# Rusql Alchemy: A Django-Inspired ORM for Rust

Welcome to Rusql Alchemy! This project is a personal challenge to create a simple, intuitive, and powerful ORM for Rust, inspired by the fantastic Django ORM. While it started as a fun side project, it has grown into a capable library that I use for my own applications.

## ✨ Core Features

*   **Django-like Model Definitions:** Define your database models using simple Rust structs and derive macros.
*   **Simple & Expressive Query API:** Fetch, create, update, and delete records with an intuitive and chainable API.
*   **Automatic Migrations:** Keep your database schema in sync with your models effortlessly.
*   **Multi-Database Support:** Works with PostgreSQL, MySQL, SQLite, and Turso out of the box.
*   **Asynchronous from the Ground Up:** Built with `async`/`.await` for modern, non-blocking applications.

## ❗️ Runtime Compatibility

This library is built on `sqlx` and `libsql`, which are designed to work with the `tokio` async runtime. All asynchronous operations in `rusql-alchemy` must be executed within a `tokio` runtime.

Using this library in other runtimes, such as the one provided by `actix-web` (`#[actix_web::main]`), will likely result in runtime panics. Please ensure you are using `#[tokio::main]` or are manually running a `tokio` runtime.

## 🚀 Getting Started

### 1. Add Rusql Alchemy to Your Project

Depending on the database you want to use, add one of the following to your `Cargo.toml`:

**For PostgreSQL:**
```toml
[dependencies]
rusql-alchemy = { git = "0.5.2", features = ["postgres"] }
sqlx = "0.8"
tokio = { version = "1", features = ["full"] }
```

**For MySQL:**
```toml
[dependencies]
rusql-alchemy = { version = "0.5.2", features = ["mysql"] }
sqlx = "0.8"
tokio = { version = "1", features = ["full"] }
```

**For SQLite:**
```toml
[dependencies]
rusql-alchemy = { version = "0.5.2", features = ["sqlite"] }
sqlx = "0.8"
tokio = { version = "1", features = ["full"] }
```

**For Turso:**
```toml
[dependencies]
rusql-alchemy = { version = "0.5.2", features = ["turso"] }
tokio = { version = "1", features = ["full"] }
```

### 2. Define Your Models

Create your database models using simple Rust structs and the `field` derive macro. The macro automatically generates the necessary code for database interactions.

```rust
use rusql_alchemy::prelude::*;

#[derive(Debug, field, FromRow, Default)]
struct User {
    #[field(primary_key=true, auto=true)]
    id: Integer,

    #[field(unique=true)]
    name: String,

    age: Integer,

    #[field(default="user")]
    role: String
}
```

> **Note for PostgreSQL users:** For auto-incrementing primary keys, it's recommended to use the `Serial` type instead of `Integer` with `auto=true`.
> 
> ```rust
> #[derive(Debug, field, FromRow, Default)]
> struct UserPg {
>     #[field(primary_key=true)]
>     id: Serial,
>     // ... other fields
> }
> ```

### 3. Connect to Your Database & Run Migrations

Instantiate the `Database` and run your migrations.

```rust
use rusql_alchemy::prelude::*;
use rusql_alchemy::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // For PostgreSQL, MySQL or SQLite
    // let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    // let database = Database::new(&database_url).await?;

    // For local development with Turso
    let database = Database::new_local("local.db").await?;

    // Run migrations to create the necessary tables
    database.migrate().await?;

    Ok(())
}
```

> **NB:** For migrations to work correctly, the models must be imported into the binary where `database.migrate()` is called. This allows the migration system to discover your models. If your models are in a separate module (e.g., `src/models.rs`), ensure you import them:
> 
> ```rust
> // In your main.rs
> use rusql_alchemy::prelude::*;
> use rusql_alchemy::Error;
> 
> // Import your models so they can be discovered for migration.
> // The `allow(unused_imports)` attribute is useful here.
> #[allow(unused_imports)]
> use crate::models::*; // Assuming models are in `src/models.rs`
> 
> #[tokio::main]
> async fn main() -> Result<(), Error> {
>     let database = Database::new_local("local.db").await?;
>     database.migrate().await?;
>     Ok(())
> }
> ```

##  CRUD Operations

### Create

```rust
use rusql_alchemy::prelude::*;
use rusql_alchemy::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let database = Database::new_local("local.db").await?;

    let new_user = User {
        name: "John Doe".to_string(),
        age: 30,
        ..Default::default()
    };
    new_user.save(&database.conn).await?;

    // Or create directly in the database
    User::create(
        kwargs!(
            name = "Jane Doe",
            age = 28
        ),
        &database.conn,
    ).await?;

    Ok(())
}
```

### Read

```rust
use rusql_alchemy::prelude::*;
use rusql_alchemy::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let database = Database::new_local("local.db").await?;

    // Get all users
    let all_users = User::all(&database.conn).await?;
    println!("All users: {:?}", all_users);

    // Get a single user
    if let Some(user) = User::get(kwargs!(name == "John Doe"), &database.conn).await? {
        println!("Found user: {:?}", user);
    }

    // Filter for multiple users
    let young_users = User::filter(kwargs!(age < 30), &database.conn).await?;
    println!("Young users: {:?}", young_users);

    Ok(())
}
```

### Update

```rust
use rusql_alchemy::prelude::*;
use rusql_alchemy::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let database = Database::new_local("local.db").await?;

    if let Some(mut user) = User::get(kwargs!(name == "John Doe"), &database.conn).await? {
        user.role = "admin".into();
        user.update(&database.conn).await?;
    }

    Ok(())
}
```

### Delete

```rust
use rusql_alchemy::prelude::*;
use rusql_alchemy::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let database = Database::new_local("local.db").await?;

    // Delete a single user
    if let Some(user) = User::get(kwargs!(name == "John Doe"), &database.conn).await? {
        user.delete(&database.conn).await?;
    }

    // Delete all users
    let all_users = User::all(&database.conn).await?;
    all_users.delete(&database.conn).await?;

    Ok(())
}
```

## A Personal Challenge

This project is, first and foremost, a personal challenge and a learning experience. It's a testament to the power and flexibility of Rust, and I'm proud of how far it has come. I hope you find it useful, and I welcome any feedback or contributions from the community.
