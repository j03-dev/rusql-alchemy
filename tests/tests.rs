#![allow(dead_code)]

use rusql_alchemy::prelude::*;

async fn conn(_name: &str) -> Result<Connection, rusql_alchemy::Error> {
    #[cfg(feature = "turso")]
    let database = Database::new_local(":memory:").await?;

    #[cfg(not(feature = "turso"))]
    let database = Database::new(&format!("sqlite:file:{_name}?mode=memory&cache=shared")).await?;

    database.up().await?;
    Ok(database.conn)
}

#[cfg(not(feature = "turso"))]
#[derive(Model, Clone, FromRow, Debug)]
struct User {
    #[field(primary_key = true, auto = true)]
    id: Option<Integer>,
    name: String,
    #[field(default = "user")]
    role: Option<String>,
    #[field(default = "now")]
    at: DateTime,
}

#[cfg(feature = "turso")]
#[derive(Model, Clone, serde::Deserialize, Debug)]
struct User {
    #[field(primary_key = true, auto = true)]
    id: Option<Integer>,
    #[field(unique = true)]
    name: String,
    #[field(default = "user")]
    role: Option<String>,
    #[field(default = "now")]
    at: DateTime,
}

#[cfg(not(feature = "turso"))]
#[derive(Model, Clone, FromRow, Debug)]
struct Profile {
    #[field(primary_key = true, auto = true)]
    profile_id: Option<Integer>,
    #[field(foreign_key=User.id, on_delete="cascade")]
    user_id: Integer,
    bio: String,
    #[field(default = "now")]
    created_at: DateTime,
}

#[cfg(feature = "turso")]
#[derive(Model, Clone, serde::Deserialize, Debug)]
struct Profile {
    #[field(primary_key = true, auto = true)]
    profile_id: Option<Integer>,
    #[field(foreign_key=User.id, on_delete="cascade")]
    user_id: Integer,
    bio: String,
    #[field(default = "now")]
    created_at: DateTime,
}

#[cfg(not(feature = "turso"))]
#[derive(Model, Clone, FromRow, Debug)]
struct Product {
    #[field(primary_key = true, auto = true)]
    id: Option<Integer>,
    #[field(foreign_key=User.id, on_delete="cascade")]
    user_id: Integer,
    name: String,
    price: Float,
}

#[cfg(feature = "turso")]
#[derive(Model, Clone, serde::Deserialize, Debug)]
struct Product {
    #[field(primary_key = true, auto = true)]
    id: Option<Integer>,
    #[field(foreign_key=User.id, on_delete="cascade")]
    user_id: Integer,
    name: String,
    price: Float,
}

#[cfg(not(feature = "turso"))]
#[derive(Model, Clone, FromRow, Debug)]
struct Event {
    #[field(primary_key = true, auto = true)]
    id: Option<Integer>,
    name: String,
    note: Option<String>,
    #[field(default = "now")]
    created_at: Option<DateTime>,
}

#[cfg(feature = "turso")]
#[derive(Model, Clone, serde::Deserialize, Debug)]
struct Event {
    #[field(primary_key = true, auto = true)]
    id: Option<Integer>,
    name: String,
    note: Option<String>,
    #[field(default = "now")]
    created_at: Option<DateTime>,
}

#[tokio::test]
async fn test_main() {
    // Setup
    let ref conn = conn("test_main").await.unwrap();

    // Create
    let user = User {
        name: "John".to_string(),
        ..Default::default()
    };
    let r = user.save(conn).await;
    assert!(r.is_ok(), "{:?}", r);
    let new_user = r.unwrap();
    assert_eq!(new_user.id, Some(1));
    assert_eq!(new_user.role, Some("user".to_string()));
    let r = User::create(kwargs!(name = "Doe"), conn).await;
    assert!(r.is_ok(), "{:?}", r);
    let new_user = r.unwrap();
    assert_eq!(new_user.id, Some(2));

    // Get
    let result = User::get(kwargs!(name = "John"), conn).await;
    assert!(result.is_ok(), "{:?}", result);
    let user = result.unwrap();
    assert!(user.is_some());
    let u = user.unwrap();
    assert_eq!(u.name, "John");
    assert_eq!(u.role, Some("user".to_string()));

    // Filter
    let results = User::filter(kwargs!(role = "user"), conn).await;
    assert!(results.is_ok(), "{:?}", results);
    let users = results.unwrap();
    assert!(!users.is_empty());

    // Update
    let mut user_to_update = User::get(kwargs!(name = "John"), conn)
        .await
        .unwrap()
        .unwrap();
    user_to_update.role = Some("admin".to_owned());
    let r = user_to_update.update(&conn).await;
    assert!(r.is_ok(), "{:?}", r);

    let updated_user = User::get(kwargs!(role == "admin"), conn)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated_user.role, Some("admin".to_string()));

    // Delete
    let r = updated_user.delete(conn).await;
    assert!(r.is_ok(), "{:?}", r);

    let deleted_user = User::get(kwargs!(role == "admin"), conn).await.unwrap();
    assert!(deleted_user.is_none());
}

#[tokio::test]
async fn test_join() {
    // Setup
    let ref conn = conn("test_join").await.unwrap();

    // Create User
    let r = User::create(kwargs!(name = "Jane"), conn).await;
    assert!(r.is_ok(), "{:?}", r);

    // Get User
    let user = User::get(kwargs!(name = "Jane"), conn)
        .await
        .unwrap()
        .unwrap();

    // Create Profile
    let r = Profile::create(kwargs!(user_id = user.id, bio = "Loves Rust"), conn).await;
    assert!(r.is_ok(), "{:?}", r);

    // Join with new syntax
    let results: Vec<User> = select!(User, Profile)
        .inner_join::<User, Profile>(kwargs!(User.id == Profile.user_id))
        .fetch_all(conn)
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    let joined_user = results.first().unwrap();
    assert_eq!(joined_user.name, "Jane");

    let profile: Profile = select!(Profile)
        .r#where(kwargs!(Profile.profile_id = 1))
        .fetch_one(conn)
        .await
        .unwrap();
    assert_eq!(profile.bio, "Loves Rust");
}

#[tokio::test]
async fn test_join_duplicate_column_names() {
    let ref conn = conn("test_join_duplicate_column_names").await.unwrap();

    let user = User::create(kwargs!(name = "Jane"), conn).await.unwrap();
    let product = Product::create(
        kwargs!(user_id = user.id, name = "Rust book", price = 42.5),
        conn,
    )
    .await
    .unwrap();

    let results: Vec<(User, Product)> = select!(User, Product)
        .inner_join::<User, Product>(kwargs!(User.id == Product.user_id))
        .fetch_all(conn)
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    let (joined_user, joined_product) = &results[0];
    assert_eq!(joined_user.id, user.id);
    assert_eq!(joined_user.name, "Jane");
    assert_eq!(joined_product.id, product.id);
    assert_eq!(joined_product.user_id, user.id.unwrap());
    assert_eq!(joined_product.name, "Rust book");
    assert_eq!(joined_product.price, 42.5);

    let one: (User, Product) = select!(User, Product)
        .inner_join::<User, Product>(kwargs!(User.id == Product.user_id))
        .fetch_one(conn)
        .await
        .unwrap();
    assert_eq!(one.0.id, user.id);
    assert_eq!(one.1.id, product.id);

    let optional: Option<(User, Product)> = select!(User, Product)
        .inner_join::<User, Product>(kwargs!(User.id == Product.user_id))
        .fetch_optional(conn)
        .await
        .unwrap();
    assert!(optional.is_some());

    let products: Vec<Product> = select!(User, Product)
        .inner_join::<User, Product>(kwargs!(User.id == Product.user_id))
        .fetch_all(conn)
        .await
        .unwrap();
    assert_eq!(products.len(), 1);
    assert_eq!(products[0].id, product.id);
    assert_eq!(products[0].name, "Rust book");

    let one_product: Product = select!(User, Product)
        .inner_join::<User, Product>(kwargs!(User.id == Product.user_id))
        .fetch_one(conn)
        .await
        .unwrap();
    assert_eq!(one_product.id, product.id);

    let reversed: Vec<(Product, User)> = select!(User, Product)
        .inner_join::<User, Product>(kwargs!(User.id == Product.user_id))
        .fetch_all(conn)
        .await
        .unwrap();
    assert_eq!(reversed[0].0.id, product.id);
    assert_eq!(reversed[0].1.id, user.id);
}

#[tokio::test]
async fn test_left_join_optional_model() {
    let ref conn = conn("test_left_join_optional_model").await.unwrap();

    let user = User::create(kwargs!(name = "Jane"), conn).await.unwrap();
    let lonely = User::create(kwargs!(name = "Lonely"), conn).await.unwrap();
    Product::create(
        kwargs!(user_id = user.id, name = "Rust book", price = 42.5),
        conn,
    )
    .await
    .unwrap();

    let results: Vec<(User, Option<Product>)> = select!(User, Product)
        .left_join::<User, Product>(kwargs!(User.id == Product.user_id))
        .fetch_all(conn)
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    for (joined_user, product) in results {
        if joined_user.id == lonely.id {
            assert!(product.is_none());
        } else {
            assert_eq!(joined_user.id, user.id);
            assert_eq!(product.unwrap().name, "Rust book");
        }
    }
}

#[tokio::test]
async fn test_default_option_field_is_not_nullable() {
    let ref conn = conn("test_default_option_field").await.unwrap();

    assert!(Event::SCHEMA.contains("created_at varchar(40) default current_timestamp not null"));
    assert!(!Event::SCHEMA.contains("note varchar(255) not null"));

    let event = Event {
        id: None,
        name: "Launch".to_string(),
        note: None,
        created_at: None,
    };
    let saved = event.save(conn).await.unwrap();
    assert_eq!(saved.id, Some(1));
    assert!(saved.created_at.is_some());

    let explicit = Event {
        id: None,
        name: "Retro".to_string(),
        note: None,
        created_at: Some("2000-01-01 00:00:00".to_string()),
    };
    let mut saved = explicit.save(conn).await.unwrap();
    assert_eq!(saved.created_at.as_deref(), Some("2000-01-01 00:00:00"));

    saved.name = "Retro updated".to_string();
    saved.created_at = None;
    saved.update(conn).await.unwrap();

    let updated = Event::get(kwargs!(name = "Retro updated"), conn)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.created_at.as_deref(), Some("2000-01-01 00:00:00"));
}
