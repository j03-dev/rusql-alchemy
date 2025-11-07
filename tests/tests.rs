use rusql_alchemy::prelude::*;

#[allow(dead_code)]
async fn setup_database() -> Database {
    #[cfg(not(feature = "turso"))]
    {
        Database::new("sqlite:file:cache?mode=memory&cache=shared")
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
#[derive(Model, Clone, FromRow, Debug)]
struct User {
    #[field(primary_key = true, auto = true)]
    id: Option<Integer>,

    name: String,

    #[field(default = "user")]
    role: String,
}

#[cfg(feature = "turso")]
#[derive(Model, Clone, serde::Deserialize, Debug)]
struct User {
    #[field(primary_key = true, auto = true)]
    id: Option<Integer>,

    #[field(unique = true)]
    name: String,

    #[field(default = "user")]
    role: String,
}

#[cfg(not(feature = "turso"))]
#[derive(Model, Clone, FromRow, Debug)]
struct Profile {
    #[field(primary_key = true, auto = true)]
    profile_id: Option<Integer>,
    #[field(foreign_key=User.id)]
    user_id: Integer,
    bio: String,
}

#[cfg(feature = "turso")]
#[derive(Model, Clone, serde::Deserialize, Debug)]
struct Profile {
    #[field(primary_key = true, auto = true)]
    profile_id: Option<Integer>,
    #[field(foreign_key=User.id)]
    user_id: Integer,
    bio: String,
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
    let r = User::create(kwargs!(name = "Doe"), &database.conn).await;
    assert!(r.is_ok());

    // Get
    let result = User::get(kwargs!(name = "John"), &database.conn).await;
    assert!(result.is_ok());
    let user = result.unwrap();
    assert!(user.is_some());
    let u = user.unwrap();
    assert_eq!(u.name, "John");
    assert_eq!(u.role, "user");

    // Filter
    let results = User::filter(kwargs!(role = "user"), &database.conn).await;
    assert!(results.is_ok());
    let users = results.unwrap();
    assert!(!users.is_empty());

    // Update
    let mut user_to_update = User::get(kwargs!(name = "John"), &database.conn)
        .await
        .unwrap()
        .unwrap();
    user_to_update.role = "admin".to_owned();
    let r = user_to_update.update(&database.conn).await;
    assert!(r.is_ok());

    let updated_user = User::get(kwargs!(role == "admin"), &database.conn)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated_user.role, "admin");

    // Delete
    let r = updated_user.delete(&database.conn).await;
    assert!(r.is_ok());

    let deleted_user = User::get(kwargs!(role == "admin"), &database.conn).await.unwrap();
    assert!(deleted_user.is_none());
}

#[tokio::test]
async fn test_join() {
    // Setup
    let database = setup_database().await;

    // Migrate
    let result = database.migrate().await;
    assert!(result.is_ok());

    // Create User
    let r = User::create(kwargs!(name = "Jane"), &database.conn).await;
    assert!(r.is_ok());

    // Get User
    let user = User::get(kwargs!(name = "Jane"), &database.conn)
        .await
        .unwrap()
        .unwrap();

    // Create Profile
    let r = Profile::create(
        kwargs!(user_id = user.id.unwrap(), bio = "Loves Rust"),
        &database.conn,
    )
    .await;
    assert!(r.is_ok());

    // Join
    let results = select!(User, Profile)
        .join::<User>(
            JoinType::Inner,
            Profile::NAME,
            kwargs!(User.id = Profile.user_id),
            &database.conn,
        )
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    let joined_user = results.first().unwrap();
    assert_eq!(joined_user.name, "Jane");
}
