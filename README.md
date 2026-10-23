# Rusql Alchemy: A Django-Inspired ORM for Rust

Welcome to Rusql Alchemy! This project is a personal challenge to create a simple, intuitive, and powerful ORM for Rust, inspired by the fantastic Django ORM. While it started as a fun side project, it has grown into a capable library that I use for my own applications.

## ✨ Core Features

*   **Django-like Model Definitions:** Define your database models using simple Rust structs and derive macros.
*   **Simple & Expressive Query API:** Fetch, create, update, and delete records with an intuitive and chainable API.
*   **Schema Sync:** Keep your database schema in sync with your models with a single `database.up()` call.
*   **Versioned Migrations:** The companion `rusql-alchemy-migration` crate diffs your models against the last snapshot and writes numbered `up.sql` / `down.sql` migrations, then applies only the ones you have not run yet — with checksums to catch edited migrations.
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
rusql-alchemy = { version = "0.6.0", default-features = false, features = ["postgres"] }
sqlx = "0.9"
tokio = { version = "1", features = ["full"] }
```

**For MySQL:**
```toml
[dependencies]
rusql-alchemy = { version = "0.6.0", default-features = false, features = ["mysql"] }
sqlx = "0.9"
tokio = { version = "1", features = ["full"] }
```

**For SQLite:**
```toml
[dependencies]
rusql-alchemy = "0.6.0" 
sqlx = "0.9"
tokio = { version = "1", features = ["full"] }
```

**For Turso:**
```toml
[dependencies]
rusql-alchemy = { version = "0.6.0", default-features = false, features = ["turso"] }
serde = "1.0"
tokio = { version = "1", features = ["full"] }
```

**Optional — versioned migrations:**
```toml
[dependencies]
rusql-alchemy-migration = { version = "0.6.0", default-features = false, features = ["sqlite"] }
```
> `rusql-alchemy-migration` ships `sqlite` (default) and `turso` features — pick the same one you chose for `rusql-alchemy`. See [Versioned Migrations](#versioned-migrations-rusql-alchemy-migration) below.

### 2. Define Your Models

Create your database models using simple Rust structs and the `field` derive macro. The macro automatically generates the necessary code for database interactions.

When using an auto-incrementing primary key (`auto=true`), it is recommended to use `Option<Integer>` for the field type. This allows the model to represent a record that has not yet been inserted into the database (where the ID would be `None`).

```rust
use rusql_alchemy::prelude::*;

#[derive(Debug, Clone, Model, FromRow)]
struct User {
    #[field(primary_key=true, auto=true)]
    id: Option<Integer>,

    #[field(unique=true)]
    name: String,

    age: Integer,

    #[field(default="user")]
    role: String
}
```

> **Defaults and `Option`:** A field with `#[field(default = ...)]` is created as `NOT NULL`, even when its Rust type is `Option<T>`. `None` means "not provided": `save()` omits the column so the SQL default is applied, and `update()` keeps the existing value. Use `Option<T>` **without** a default when you need a genuinely nullable column.
>
> With Serde/Actix, missing JSON fields still need `#[serde(default)]` (on the struct) if you want the generated Rust `Default` to fill them before saving.

> **Note for PostgreSQL users:** For auto-incrementing primary keys, it's recommended to use the `Serial` type instead of `Integer` with `auto=true`.
> 
> ```rust
> #[derive(Debug, Clone, Model, FromRow)]
> struct UserPg {
>     #[field(primary_key=true)]
>     id: Serial,
>     // ... other fields
> }
> ```

### 3. Connect to Your Database & Apply Your Schema

Instantiate the `Database` and apply the schema declared by your models.

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

    // Create the tables declared by your models (idempotent)
    database.up().await?;

    Ok(())
}
```

> **NB:** For `database.up()` to see your models, they must be imported into the binary that calls it. Each `#[derive(Model)]` registers its schema through `inventory`, so an unused import is enough. If your models live in a separate module (e.g., `src/models.rs`), import them:
>
> ```rust
> // In your main.rs
> use rusql_alchemy::prelude::*;
> use rusql_alchemy::Error;
>
> // Import your models so they can be registered for schema sync.
> // The `allow(unused_imports)` attribute is useful here.
> #[allow(unused_imports)]
> use crate::models::*; // Assuming models are in `src/models.rs`
>
> #[tokio::main]
> async fn main() -> Result<(), Error> {
>     let database = Database::new_local("local.db").await?;
>     database.up().await?;
>     Ok(())
> }
> ```

### Schema Sync (`Database.up`)

`database.up()` executes the `SCHEMA` constant of every registered model — one `create table if not exists ...` statement per model — in registration order.

*   Idempotent: running it twice is harmless, so it is safe to call on every startup.
*   It only creates tables. It does **not** alter existing tables when a model changes; for that, use [versioned migrations](#versioned-migrations-rusql-alchemy-migration) below.

```rust
use rusql_alchemy::prelude::*;
use rusql_alchemy::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let database = Database::new("sqlite::memory:").await?;
    database.up().await?;
    println!("All registered model schemas applied.");
    Ok(())
}
```

### Model Trait for Schema Management

When you derive `#[derive(Model)]` for your struct, it implements the `Model` trait and registers the model's schema with `inventory`.

*   **`Model::SCHEMA`**: A `&'static str` containing the `CREATE TABLE IF NOT EXISTS` statement for the model.
*   **`Model::NAME`**: A `&'static str` representing the name of the database table.
*   **`Model::PK`**: A `&'static str` representing the primary key column name.

The derive also provides `save`, `update` and `delete` implementations, a `Default` impl for every field, and the schema registration used by `database.up()`.

```rust
use rusql_alchemy::prelude::*;
use sqlx::FromRow; // Required for sqlx-based features

#[derive(Debug, Clone, Model, FromRow)]
struct Product {
    #[field(primary_key=true, auto=true)]
    id: Option<Integer>,
    name: String,
    price: Float,
}

#[tokio::main]
async fn main() -> Result<(), rusql_alchemy::Error> {
    let database = Database::new("sqlite::memory:").await?;

    // Create the 'products' table along with every other registered model
    database.up().await?;

    println!("Table '{}' created (PK: '{}').", Product::NAME, Product::PK);

    Ok(())
}
```

### Versioned Migrations (`rusql-alchemy-migration`)

`Database::up()` is great for prototypes and fresh databases, but it never changes an existing table. The companion crate **`rusql-alchemy-migration`** adds Django-style, file-based migrations:

*   **Diff your models** against the schema of the last migration and write a new, numbered migration directory.
*   **Apply only pending migrations**, in order, recording them in a history table.
*   **Detect tampering**: every applied migration is checksummed, and editing a migration that already ran is an error.

```toml
[dependencies]
rusql-alchemy-migration = { version = "0.6.0", default-features = false, features = ["sqlite"] }
```

> The crate exposes `sqlite` (default) and `turso` features. Use the same backend feature you picked for `rusql-alchemy`.

#### How it works

`make_migration` collects `Model::SCHEMA` from every registered model (sorted, so registration order cannot create false diffs), compares it to the `schema.sql` snapshot stored inside the latest migration, and writes the difference:

```text
migrations/
├── 0001_first_migration/
│   ├── up.sql        <- statements to go from the previous schema to this one
│   ├── down.sql      <- the inverse, in reverse order
│   └── schema.sql    <- full schema snapshot after this migration
└── 0002_add_email/
    ├── up.sql
    ├── down.sql
    └── schema.sql
```

`migrate` then walks the directory from oldest to newest and runs every migration whose version is missing from the `RusqlAlchemyMigration` history table.

#### API

*   **`migration::make_migration(directory, name)`** — writes a migration into `directory` if the models changed since the last snapshot, and returns its directory name. `name` is free text (`"first migration"` → `0001_first_migration`) and defaults to `schema_change`. Returns `None` when nothing changed.
*   **`migration::migrate(database, directory)`** — applies every pending migration, each inside a transaction, and records it. Fails if an already-applied migration was edited afterwards.
*   **`migration::rollback(database, directory)`** — reverts the latest applied migration (runs its `down.sql`, removes its history row, all in one transaction). Returns its name, or `None` if nothing is applied. Fails if the migration's directory is missing or it was edited after being applied.
*   **`migration::cli(database, directory, args)`** — a small command line over all: `make-migration [name]`, `migrate`, `rollback`, `help`.

#### Example: a `main` that doubles as a CLI

```rust
use rusql_alchemy::prelude::*;
use rusql_alchemy::Error;

// Import your models so they are registered for diffing.
#[allow(unused_imports)]
use crate::models::*;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let database = Database::new_local("local.db").await?;

    // Usage: <program> make-migration [name] | migrate | rollback | help
    rusql_alchemy_migration::cli(&database, "migrations", std::env::args().skip(1)).await
}
```

Or call the functions directly:

```rust
use rusql_alchemy::prelude::*;
use rusql_alchemy::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let database = Database::new_local("local.db").await?;

    // 1. Generate a migration after changing your models
    match rusql_alchemy_migration::make_migration("migrations", Some("first migration"))? {
        Some(created) => println!("Created migration {created}"),
        None => println!("No changes detected"),
    }

    // 2. Apply everything that has not run yet
    rusql_alchemy_migration::migrate(&database, "migrations").await?;

    Ok(())
}
```

> **Rollback:** each migration also gets a `down.sql` with the inverse statements. You can revert the latest applied migration with `rollback` (or `migration::rollback(...)`). The command checks the migration was not modified after it was applied.

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


## JOIN Operations

Rusql Alchemy supports `INNER`, `LEFT` and `RIGHT` joins, allowing you to query data from multiple tables at once.

First, define the models with a foreign key relationship. Assuming the `User` model from before, we can define a `Profile` model:

```rust
use rusql_alchemy::prelude::*;

#[derive(Debug, Clone, Model, FromRow)]
struct Profile {
    #[field(primary_key=true, auto=true)]
    id: Option<Integer>,

    #[field(foreign_key=User.id)]
    user_id: Integer,

    bio: String,
}
```

Then, you can use the `select!` macro to build the query. `inner_join::<Base, Join>(on)` adds the join clause, and `fetch_all::<T>()` (or `fetch_one` / `fetch_optional`) executes it and returns rows decoded as `T`.

```rust
use rusql_alchemy::prelude::*;
use rusql_alchemy::Error;

// Assuming User and Profile models are defined as above
#[tokio::main]
async fn main() -> Result<(), Error> {
    let database = Database::new_local("local.db").await?;
    database.up().await?;

    // Create a user
    User::create(kwargs!(name = "Jane", age = 25), &database.conn).await?;

    // Get the user to access the auto-generated id
    let user = User::get(kwargs!(name == "Jane"), &database.conn)
        .await?
        .expect("User should exist");

    // Create a profile for the user
    Profile::create(
        kwargs!(user_id = user.id.unwrap(), bio = "Loves Rust"),
        &database.conn,
    ).await?;

    // Perform an inner join to get Users
    let results: Vec<User> = select!(User, Profile)
        .inner_join::<User, Profile>(kwargs!(User.id == Profile.user_id))
        .fetch_all(&database.conn)
        .await?;

    println!("Joined users: {:?}", results);

    Ok(())
}
```

## A Personal Challenge

This project is, first and foremost, a personal challenge and a learning experience. It's a testament to the power and flexibility of Rust, and I'm proud of how far it has come. I hope you find it useful, and I welcome any feedback or contributions from the community.
