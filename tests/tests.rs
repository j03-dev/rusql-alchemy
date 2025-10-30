use rusql_alchemy::prelude::*;

#[allow(dead_code)]
async fn setup_database() -> Database {
    #[cfg(not(feature = "turso"))]
    {
        Database::new("sqlite:file:db?mode=memory&cache=shared")
            .await
            .expect("failed to init database")
    }
    #[cfg(feature = "turso")]
    {
        Database::new_local(":memory:")
            .await
            .expect("failed to init database")
    }
}

#[cfg(not(feature = "turso"))]
#[derive(Model, Clone, FromRow)]
struct User {
    #[field(primary_key = true, auto = true)]
    id: Option<Integer>,

    name: String,

    #[field(default = "user")]
    role: String,
}

#[cfg(feature = "turso")]
#[derive(Model, Clone, serde::Deserialize)]
struct User {
    #[field(primary_key = true, auto = true)]
    id: Option<Integer>,

    #[field(unique = true)]
    name: String,

    #[field(default = "user")]
    role: String,
}

#[tokio::test]
async fn test_main() {
    // Setup
    let database = setup_database().await;

    // Migrate
    let result = database.migrate().await;
    assert!(result.is_ok());

    // Create
    let r = User::create(kwargs!(name = "John"), &database.conn).await;
    assert!(r.is_ok());

    // Get
    let result = User::get(kwargs!(name = "John"), &database.conn).await;
    assert!(result.is_ok());
    let user = result.unwrap();
    assert!(user.is_some());
    let u = user.unwrap();
    assert_eq!(u.id, Some(1));
    assert_eq!(u.name, "John");
    assert_eq!(u.role, "user");

    // Update
    let mut user_to_update = User::get(kwargs!(name = "John"), &database.conn)
        .await
        .unwrap()
        .unwrap();
    user_to_update.role = "admin".to_owned();
    let r = user_to_update.update(&database.conn).await;
    assert!(r.is_ok());

    let updated_user = User::get(kwargs!(id = 1), &database.conn)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated_user.role, "admin");

    // Delete
    let r = updated_user.delete(&database.conn).await;
    assert!(r.is_ok());

    let deleted_user = User::get(kwargs!(id = 1), &database.conn).await.unwrap();
    assert!(deleted_user.is_none());
}
