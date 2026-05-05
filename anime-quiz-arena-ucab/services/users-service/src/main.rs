use std::env;

use axum::{routing::get, Json, Router};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::json;
use sqlx::{PgPool, Row};
use tokio::net::TcpListener;
use tonic::{transport::Server, Request, Response, Status};
use tracing::{error, info, warn};
use uuid::Uuid;

pub mod users {
    tonic::include_proto!("animequiz.users.v1");
}

use users::users_service_server::{UsersService, UsersServiceServer};
use users::{
    CreateUserRequest, GetUserRequest, LoginBasicRequest, LoginBasicResponse, User, UserResponse,
};

#[derive(Clone)]
struct UsersSvc {
    pool: PgPool,
}

async fn init_db(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE,
            password TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT now()
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        ALTER TABLE users
        ALTER COLUMN created_at TYPE TIMESTAMPTZ
        USING created_at AT TIME ZONE 'UTC'
        "#,
    )
    .execute(pool)
    .await
    .or_else(|err| {
        warn!(error = %err, "created_at TIMESTAMPTZ migration skipped or already compatible");
        Ok::<_, sqlx::Error>(sqlx::postgres::PgQueryResult::default())
    })?;

    Ok(())
}

fn decode_created_at(row: &sqlx::postgres::PgRow) -> Result<String, sqlx::Error> {
    if let Ok(value) = row.try_get::<DateTime<Utc>, _>("created_at") {
        return Ok(value.to_rfc3339());
    }

    let value = row.try_get::<NaiveDateTime, _>("created_at")?;
    Ok(DateTime::<Utc>::from_naive_utc_and_offset(value, Utc).to_rfc3339())
}

fn decode_user_without_password(row: &sqlx::postgres::PgRow) -> Result<User, sqlx::Error> {
    Ok(User {
        id: row.try_get("id")?,
        username: row.try_get("username")?,
        email: row.try_get("email")?,
        created_at: decode_created_at(row)?,
    })
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    match error {
        sqlx::Error::Database(db_error) => db_error.code().as_deref() == Some("23505"),
        _ => false,
    }
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "users-service"
    }))
}

#[tonic::async_trait]
impl UsersService for UsersSvc {
    async fn create_user(
        &self,
        request: Request<CreateUserRequest>,
    ) -> Result<Response<UserResponse>, Status> {
        let request = request.into_inner();

        if request.username.trim().is_empty() {
            return Err(Status::invalid_argument("username is required"));
        }

        if request.email.trim().is_empty() {
            return Err(Status::invalid_argument("email is required"));
        }

        if request.password.trim().is_empty() {
            return Err(Status::invalid_argument("password is required"));
        }

        let id = Uuid::new_v4().to_string();

        let row = sqlx::query(
            r#"
            INSERT INTO users (id, username, email, password)
            VALUES ($1, $2, $3, $4)
            RETURNING id, username, email, created_at
            "#,
        )
        .bind(&id)
        .bind(request.username.trim())
        .bind(request.email.trim())
        .bind(request.password)
        .fetch_one(&self.pool)
        .await
        .map_err(|err| {
            if is_unique_violation(&err) {
                Status::already_exists("email already exists")
            } else {
                error!(error = %err, "Failed to create user");
                Status::unavailable("database error")
            }
        })?;

        let user = decode_user_without_password(&row).map_err(|err| {
            error!(error = %err, "Failed to decode created user row");
            Status::unavailable("database decode error")
        })?;

        info!(id = %user.id, email = %user.email, "User created");

        Ok(Response::new(UserResponse { user: Some(user) }))
    }

    async fn get_user(
        &self,
        request: Request<GetUserRequest>,
    ) -> Result<Response<UserResponse>, Status> {
        let request = request.into_inner();

        if request.id.trim().is_empty() {
            return Err(Status::invalid_argument("id is required"));
        }

        let row = sqlx::query(
            r#"
            SELECT id, username, email, created_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(request.id.trim())
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| {
            error!(error = %err, "Failed to get user");
            Status::unavailable("database error")
        })?
        .ok_or_else(|| Status::not_found("user not found"))?;

        let user = decode_user_without_password(&row).map_err(|err| {
            error!(error = %err, "Failed to decode user row");
            Status::unavailable("database decode error")
        })?;

        Ok(Response::new(UserResponse { user: Some(user) }))
    }

    async fn login_basic(
        &self,
        request: Request<LoginBasicRequest>,
    ) -> Result<Response<LoginBasicResponse>, Status> {
        let request = request.into_inner();

        if request.email.trim().is_empty() {
            return Err(Status::invalid_argument("email is required"));
        }

        if request.password.trim().is_empty() {
            return Err(Status::invalid_argument("password is required"));
        }

        info!(email = %request.email, "LoginBasic attempt");

        let row = sqlx::query(
            r#"
            SELECT id, username, email, password, created_at
            FROM users
            WHERE email = $1
            "#,
        )
        .bind(request.email.trim())
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| {
            error!(error = %err, "Failed during LoginBasic lookup");
            Status::unavailable("database error")
        })?
        .ok_or_else(|| {
            warn!(email = %request.email, "LoginBasic user not found");
            Status::unauthenticated("invalid credentials")
        })?;

        let stored_password: String = row.try_get("password").map_err(|err| {
            error!(error = %err, "Failed to decode password column");
            Status::unavailable("database decode error")
        })?;

        if stored_password != request.password {
            warn!(email = %request.email, "LoginBasic password mismatch");
            return Err(Status::unauthenticated("invalid credentials"));
        }

        let user = decode_user_without_password(&row).map_err(|err| {
            error!(error = %err, "Failed to decode LoginBasic user row");
            Status::unavailable("database decode error")
        })?;

        info!(id = %user.id, email = %user.email, "LoginBasic success");

        Ok(Response::new(LoginBasicResponse {
            success: true,
            user: Some(user),
        }))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let grpc_port = env::var("GRPC_PORT").unwrap_or_else(|_| "50051".to_string());
    let grpc_addr = format!("0.0.0.0:{grpc_port}").parse()?;

    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@users-db:5432/users_db".to_string());

    info!("Starting users-service gRPC on {}", grpc_addr);

    let pool = PgPool::connect(&database_url).await.map_err(|err| {
        error!(error = %err, "Failed to connect to PostgreSQL");
        err
    })?;

    info!("Connected to PostgreSQL");

    init_db(&pool).await.map_err(|err| {
        error!(error = %err, "Failed creating users table");
        err
    })?;

    info!("Users table ready");

    let service = UsersSvc { pool };

    let grpc_server = async move {
        Server::builder()
            .add_service(UsersServiceServer::new(service))
            .serve(grpc_addr)
            .await
            .map_err(anyhow::Error::from)
    };

    if let Ok(port) = env::var("PORT") {
        let http_addr = format!("0.0.0.0:{port}").parse()?;
        let app = Router::new().route("/health", get(health));

        info!("Starting users-service HTTP health on {}", http_addr);

        let http_server = async move {
            let listener = TcpListener::bind(http_addr).await?;
            axum::serve(listener, app).await?;
            Ok::<(), anyhow::Error>(())
        };

        tokio::try_join!(grpc_server, http_server)?;
    } else {
        grpc_server.await?;
    }

    Ok(())
}