use std::env;

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use tonic::{transport::Server, Request, Response, Status};
use tracing::{error, info};
use uuid::Uuid;

pub mod users {
    tonic::include_proto!("animequiz.users.v1");
}

use users::users_service_server::{UsersService, UsersServiceServer};
use users::{
    CreateUserRequest, CreateUserResponse, GetUserRequest, GetUserResponse, LoginBasicRequest,
    LoginBasicResponse, User,
};

#[derive(Clone)]
struct UsersSvc {
    pool: PgPool,
}

#[tonic::async_trait]
impl UsersService for UsersSvc {
    async fn create_user(
        &self,
        request: Request<CreateUserRequest>,
    ) -> Result<Response<CreateUserResponse>, Status> {
        let req = request.into_inner();

        if req.username.trim().is_empty()
            || req.email.trim().is_empty()
            || req.password.trim().is_empty()
        {
            return Err(Status::invalid_argument(
                "username, email and password must not be empty",
            ));
        }

        let user_id = Uuid::new_v4();
        let created_at = Utc::now();

        let result = sqlx::query(
            "INSERT INTO users (id, username, email, password, created_at) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(user_id)
        .bind(req.username.trim())
        .bind(req.email.trim())
        .bind(req.password)
        .bind(created_at)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => {
                info!(id = %user_id, email = %req.email, "User created");
                Ok(Response::new(CreateUserResponse {
                    user: Some(User {
                        id: user_id.to_string(),
                        username: req.username.trim().to_string(),
                        email: req.email.trim().to_string(),
                        created_at: created_at.to_rfc3339(),
                    }),
                }))
            }
            Err(sqlx::Error::Database(db_err)) => {
                if db_err.code().as_deref() == Some("23505") {
                    Err(Status::already_exists("email already exists"))
                } else {
                    Err(Status::unavailable(format!("database error: {db_err}")))
                }
            }
            Err(e) => Err(Status::unavailable(format!("database error: {e}"))),
        }
    }

    async fn get_user(
        &self,
        request: Request<GetUserRequest>,
    ) -> Result<Response<GetUserResponse>, Status> {
        let req = request.into_inner();
        if req.id.trim().is_empty() {
            return Err(Status::invalid_argument("id must not be empty"));
        }

        let user_id = Uuid::parse_str(req.id.trim())
            .map_err(|_| Status::invalid_argument("id must be a valid UUID"))?;

        let row = sqlx::query("SELECT id, username, email, created_at FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| Status::unavailable(format!("database error: {e}")))?;

        let row = row.ok_or_else(|| Status::not_found("user not found"))?;

        let id: Uuid = row.get("id");
        let username: String = row.get("username");
        let email: String = row.get("email");
        let created_at: DateTime<Utc> = row.get("created_at");

        info!(id = %id, "User queried");

        Ok(Response::new(GetUserResponse {
            user: Some(User {
                id: id.to_string(),
                username,
                email,
                created_at: created_at.to_rfc3339(),
            }),
        }))
    }

    async fn login_basic(
        &self,
        request: Request<LoginBasicRequest>,
    ) -> Result<Response<LoginBasicResponse>, Status> {
        let req = request.into_inner();

        if req.email.trim().is_empty() || req.password.trim().is_empty() {
            return Err(Status::invalid_argument(
                "email and password must not be empty",
            ));
        }

        let row = sqlx::query(
            "SELECT id, username, email, password, created_at FROM users WHERE email = $1",
        )
        .bind(req.email.trim())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::unavailable(format!("database error: {e}")))?;

        let row = row.ok_or_else(|| Status::unauthenticated("invalid credentials"))?;

        let saved_password: String = row.get("password");
        if saved_password != req.password {
            info!(email = %req.email, "Login failed");
            return Err(Status::unauthenticated("invalid credentials"));
        }

        let id: Uuid = row.get("id");
        let username: String = row.get("username");
        let email: String = row.get("email");
        let created_at: DateTime<Utc> = row.get("created_at");

        info!(email = %email, "Login success");

        Ok(Response::new(LoginBasicResponse {
            success: true,
            user: Some(User {
                id: id.to_string(),
                username,
                email,
                created_at: created_at.to_rfc3339(),
            }),
        }))
    }
}

async fn init_db(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id UUID PRIMARY KEY,
            username VARCHAR NOT NULL,
            email VARCHAR NOT NULL UNIQUE,
            password VARCHAR NOT NULL,
            created_at TIMESTAMP NOT NULL
        )",
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let addr = "0.0.0.0:50051".parse()?;
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@users-db:5432/users_db".to_string());

    info!("Starting users-service on 0.0.0.0:50051");
    let pool = PgPool::connect(&database_url).await.map_err(|e| {
        error!(error = %e, "Failed to connect to PostgreSQL");
        e
    })?;
    info!("Connected to PostgreSQL");

    init_db(&pool).await.map_err(|e| {
        error!(error = %e, "Failed creating users table");
        e
    })?;
    info!("Users table ready");

    let service = UsersSvc { pool };

    Server::builder()
        .add_service(UsersServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
