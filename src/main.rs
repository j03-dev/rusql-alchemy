use rust_alchemy::prelude::*;

#[derive(Deserialize, Default)]
struct User {
    id: Option<i32>,
    name: String,
    email: String,
    password: String,
}

#[async_trait::async_trait]
impl Model for User {
    const SCHEMA: &'static str = r#"
    create table User (
        id integer primary key autoincrement,
        name varchar(255) not null,
        email varchar(255) not null,
        password varchar(255) not null
    );"#;
    const NAME: &'static str = "User";

    async fn save(&self, conn: &Connection) -> bool {
        Self::create(
            kwargs!(
                name = self.name,
                email = self.email,
                password = self.password
            ),
            conn,
        )
        .await
    }
}

#[tokio::main]
async fn main() {
    let conn = config::db::Database::new().await.conn;
    let user = User {
        name: "John Doe".to_string(),
        email: "johndoe@gmailcom".to_string(),
        password: "password".to_string(),
        ..Default::default()
    };

    user.save(&conn).await;
    User::create(
        kwargs!(
            name = "joe",
            email = "24nomeniavo@gmail.com",
            password = "password"
        ),
        &conn,
    )
    .await;
    let user = User::get(kwargs!(name = "John Doe"), &conn).await;
    User::filter(kwargs!(name = "John Doe", name = "joe").or(), &conn).await;
    println!("{:?}", user.id);
}
