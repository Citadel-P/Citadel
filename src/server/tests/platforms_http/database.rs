use sqlx::{Connection, PgConnection};
use uuid::Uuid;

/// Administrator list and realtime reads see every row, including other tests'
/// deliberately incomplete records. Give each fixture its own migrated database.
pub(super) struct TestDatabase {
    admin_url: String,
    name: String,
    pub(super) url: String,
}

impl TestDatabase {
    pub(super) async fn create(admin_url: String) -> Self {
        let name = format!("citadel_platform_http_{}", Uuid::now_v7().simple());
        let mut url = url::Url::parse(&admin_url).unwrap();
        url.set_path(&name);
        let mut admin = PgConnection::connect(&admin_url).await.unwrap();
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE DATABASE {name}")))
            .execute(&mut admin)
            .await
            .expect("CITADEL_PLATFORM_DATABASE_URL must allow CREATE DATABASE");
        Self {
            admin_url,
            name,
            url: url.into(),
        }
    }
}

impl Drop for TestDatabase {
    fn drop(&mut self) {
        let admin_url = self.admin_url.clone();
        let name = self.name.clone();
        // Cleanup must finish even when a test panics or its Tokio runtime is
        // shutting down. Use a separate thread to run cleanup on its own runtime.
        let cleanup = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let mut admin = PgConnection::connect(&admin_url).await?;
                    sqlx::query(sqlx::AssertSqlSafe(format!(
                        "DROP DATABASE {name} WITH (FORCE)"
                    )))
                    .execute(&mut admin)
                    .await?;
                    Ok::<_, sqlx::Error>(())
                })
        })
        .join();
        if !matches!(cleanup, Ok(Ok(()))) {
            if std::thread::panicking() {
                eprintln!("could not remove platform HTTP test database: {cleanup:?}");
            } else {
                panic!("could not remove platform HTTP test database: {cleanup:?}");
            }
        }
    }
}
