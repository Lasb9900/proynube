use std::{env, net::SocketAddr};

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgPool, Row};
use tokio::net::TcpListener;
use tonic::{transport::Server, Code, Request, Response, Status};
use tracing::{error, info, warn};
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

#[derive(Debug, Deserialize)]
struct CreateUserBody {
    username: String,
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct LoginBody {
    email: String,
    password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UserDto {
    id: String,
    username: String,
    email: String,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct UserResponseDto {
    user: Option<UserDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LoginResponseDto {
    success: bool,
    user: Option<UserDto>,
}

#[derive(Debug, Serialize)]
struct ApiErrorResponse {
    error: String,
    code: String,
}

impl From<User> for UserDto {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            created_at: user.created_at,
        }
    }
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

fn map_status_to_http(status: Status) -> (StatusCode, Json<ApiErrorResponse>) {
    let http_status = match status.code() {
        Code::InvalidArgument => StatusCode::BAD_REQUEST,
        Code::Unauthenticated => StatusCode::UNAUTHORIZED,
        Code::NotFound => StatusCode::NOT_FOUND,
        Code::AlreadyExists => StatusCode::CONFLICT,
        Code::FailedPrecondition => StatusCode::PRECONDITION_FAILED,
        Code::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };

    let code = match status.code() {
        Code::InvalidArgument => "invalid_argument",
        Code::Unauthenticated => "unauthenticated",
        Code::NotFound => "not_found",
        Code::AlreadyExists => "already_exists",
        Code::FailedPrecondition => "failed_precondition",
        Code::Unavailable => "unavailable",
        _ => "internal",
    };

    (
        http_status,
        Json(ApiErrorResponse {
            error: status.message().to_string(),
            code: code.to_string(),
        }),
    )
}

impl UsersSvc {
    async fn create_user_core(
        &self,
        username: String,
        email: String,
        password: String,
    ) -> Result<User, Status> {
        if username.trim().is_empty() {
            return Err(Status::invalid_argument("username is required"));
        }

        if email.trim().is_empty() {
            return Err(Status::invalid_argument("email is required"));
        }

        if password.trim().is_empty() {
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
        .bind(username.trim())
        .bind(email.trim())
        .bind(password)
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

        Ok(user)
    }

    async fn get_user_core(&self, id: String) -> Result<User, Status> {
        if id.trim().is_empty() {
            return Err(Status::invalid_argument("id is required"));
        }

        let row = sqlx::query(
            r#"
            SELECT id, username, email, created_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(id.trim())
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| {
            error!(error = %err, "Failed to get user");
            Status::unavailable("database error")
        })?
        .ok_or_else(|| Status::not_found("user not found"))?;

        decode_user_without_password(&row).map_err(|err| {
            error!(error = %err, "Failed to decode user row");
            Status::unavailable("database decode error")
        })
    }

    async fn login_basic_core(&self, email: String, password: String) -> Result<User, Status> {
        if email.trim().is_empty() {
            return Err(Status::invalid_argument("email is required"));
        }

        if password.trim().is_empty() {
            return Err(Status::invalid_argument("password is required"));
        }

        info!(email = %email, "LoginBasic attempt");

        let row = sqlx::query(
            r#"
            SELECT id, username, email, password, created_at
            FROM users
            WHERE email = $1
            "#,
        )
        .bind(email.trim())
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| {
            error!(error = %err, "Failed during LoginBasic lookup");
            Status::unavailable("database error")
        })?
        .ok_or_else(|| {
            warn!(email = %email, "LoginBasic user not found");
            Status::unauthenticated("invalid credentials")
        })?;

        let stored_password: String = row.try_get("password").map_err(|err| {
            error!(error = %err, "Failed to decode password column");
            Status::unavailable("database decode error")
        })?;

        if stored_password != password {
            warn!(email = %email, "LoginBasic password mismatch");
            return Err(Status::unauthenticated("invalid credentials"));
        }

        let user = decode_user_without_password(&row).map_err(|err| {
            error!(error = %err, "Failed to decode LoginBasic user row");
            Status::unavailable("database decode error")
        })?;

        info!(id = %user.id, email = %user.email, "LoginBasic success");

        Ok(user)
    }
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "users-service"
    }))
}

async fn create_user_http(
    State(service): State<UsersSvc>,
    Json(body): Json<CreateUserBody>,
) -> Result<Json<UserResponseDto>, (StatusCode, Json<ApiErrorResponse>)> {
    let user = service
        .create_user_core(body.username, body.email, body.password)
        .await
        .map_err(map_status_to_http)?;

    Ok(Json(UserResponseDto {
        user: Some(UserDto::from(user)),
    }))
}

async fn login_http(
    State(service): State<UsersSvc>,
    Json(body): Json<LoginBody>,
) -> Result<Json<LoginResponseDto>, (StatusCode, Json<ApiErrorResponse>)> {
    let user = service
        .login_basic_core(body.email, body.password)
        .await
        .map_err(map_status_to_http)?;

    Ok(Json(LoginResponseDto {
        success: true,
        user: Some(UserDto::from(user)),
    }))
}

#[tonic::async_trait]
impl UsersService for UsersSvc {
    async fn create_user(
        &self,
        request: Request<CreateUserRequest>,
    ) -> Result<Response<CreateUserResponse>, Status> {
        let request = request.into_inner();

        let user = self
            .create_user_core(request.username, request.email, request.password)
            .await?;

        Ok(Response::new(CreateUserResponse { user: Some(user) }))
    }

    async fn get_user(
        &self,
        request: Request<GetUserRequest>,
    ) -> Result<Response<GetUserResponse>, Status> {
        let request = request.into_inner();

        let user = self.get_user_core(request.id).await?;

        Ok(Response::new(GetUserResponse { user: Some(user) }))
    }

    async fn login_basic(
        &self,
        request: Request<LoginBasicRequest>,
    ) -> Result<Response<LoginBasicResponse>, Status> {
        let request = request.into_inner();

        let user = self
            .login_basic_core(request.email, request.password)
            .await?;

        Ok(Response::new(LoginBasicResponse {
            success: true,
            user: Some(user),
        }))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let grpc_port = env::var("USERS_GRPC_PORT")
        .or_else(|_| env::var("GRPC_PORT"))
        .unwrap_or_else(|_| "50051".to_string());

    let grpc_addr: SocketAddr = format!("0.0.0.0:{grpc_port}").parse()?;

    let http_port = env::var("PORT").unwrap_or_else(|_| "18051".to_string());
    let http_addr: SocketAddr = format!("0.0.0.0:{http_port}").parse()?;

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

    let grpc_service = service.clone();
    let http_service = service.clone();

    let grpc_server = async move {
        Server::builder()
            .add_service(UsersServiceServer::new(grpc_service))
            .serve(grpc_addr)
            .await
            .map_err(anyhow::Error::from)
    };

    let http_server = async move {
        let app = Router::new()
            .route("/health", get(health))
            .route("/api/users", post(create_user_http))
            .route("/api/login", post(login_http))
            .with_state(http_service);

        info!("Starting users-service HTTP on {}", http_addr);

        let listener = TcpListener::bind(http_addr).await?;
        axum::serve(listener, app).await?;

        Ok::<(), anyhow::Error>(())
    };

    tokio::try_join!(grpc_server, http_server)?;

    Ok(())
}