use sqlx::{migrate::Migrator, Connection, Executor, PgConnection, PgPool};
use std::{path::Path, thread, time};
use uuid::Uuid;

use crate::{start_container, stop_container};

/// TestPostgres contains a db connection infomation.
pub struct TestPostgres {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub dbname: String,
    pub container_id: String,
}

impl TestPostgres {
    /// creates a TestPostgres.
    pub async fn new(migration_path: impl Into<String>) -> Result<Self, anyhow::Error> {
        // config databse
        let dbname = format!("test_postgres_{}", Uuid::new_v4());
        let image = "postgres:14-alpine";
        let port = "5432";
        let user = format!("postgres_user_{}", Uuid::new_v4());
        let password = format!("postgres_password_{}", Uuid::new_v4());
        let args = &[
            "-e",
            &format!("POSTGRES_USER={}", user),
            "-e",
            &format!("POSTGRES_PASSWORD={}", password),
        ];
        let container =
            start_container(image, port, args).expect("Failed to start Postgres container");
        let test_postgres = Self {
            dbname: dbname.clone(),
            container_id: container.id,
            host: container.host,
            port: container.port,
            user,
            password,
        };
        for i in 1..=10 {
            match PgConnection::connect(&test_postgres.server_url()).await {
                Ok(conn) => {
                    conn.close().await?;
                    println!("Postgres is ready to go");
                    break;
                }
                Err(err) => {
                    if i == 10 {
                        return Err(anyhow::anyhow!(err));
                    }
                    println!("Postgres is not ready");
                    let ten_millis = time::Duration::from_secs(i);
                    thread::sleep(ten_millis);
                }
            }
        }
        let mut conn = PgConnection::connect(&test_postgres.server_url())
            .await
            .expect("Cannot connect to Postgres");

        conn.execute(format!(r#"CREATE DATABASE "{}";"#, dbname.clone()).as_str())
            .await
            .expect("Failed to create database");

        println!("Postgres created database {}", dbname.clone());
        // migrate database
        let db_pool = PgPool::connect(&test_postgres.url())
            .await
            .expect("Failed to connect to Postgres with db");

        let m = Migrator::new(Path::new(&migration_path.into()))
            .await
            .expect("Failed to migrate the database");
        m.run(&db_pool)
            .await
            .expect("Failed to migrate the database");

        println!("Postgres database {} migrated", dbname.clone());
        db_pool.close().await;

        Ok(test_postgres)
    }

    /// gets a postgres db pool.
    pub async fn get_pool(&self) -> PgPool {
        sqlx::postgres::PgPoolOptions::default()
            .max_connections(5)
            .connect(&self.url())
            .await
            .unwrap()
    }

    pub fn server_url(&self) -> String {
        if self.password.is_empty() {
            format!("postgres://{}@{}:{}", self.user, self.host, self.port)
        } else {
            format!(
                "postgres://{}:{}@{}:{}",
                self.user, self.password, self.host, self.port
            )
        }
    }

    pub fn url(&self) -> String {
        format!("{}/{}", self.server_url(), self.dbname)
    }
}

impl Drop for TestPostgres {
    fn drop(&mut self) {
        stop_container(self.container_id.clone()).expect("Failed to stop Postgres container");
        println!("Postgres container {} dropped", self.container_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_db_should_create_and_drop() {
        // create a postgres container on here
        let test_postgres = TestPostgres::new("./migrations").await.unwrap();
        let pool = test_postgres.get_pool().await;
        // insert todo
        sqlx::query("INSERT INTO todos (title) VALUES ('test')")
            .execute(&pool)
            .await
            .unwrap();

        // get todo
        let (id, title) = sqlx::query_as::<_, (i32, String)>("SELECT id, title FROM todos")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(id, 1);
        assert_eq!(title, "test");
        // drop the postgres container on here
    }
}
