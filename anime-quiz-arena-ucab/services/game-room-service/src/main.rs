use axum::{routing::get, Json, Router};
use chrono::{NaiveDateTime, Utc};
use serde_json::json;
use sqlx::Error as SqlxError;
use sqlx::{postgres::PgPoolOptions, FromRow, PgPool};
use std::{env, net::SocketAddr};
use tokio::net::TcpListener;
use tonic::{transport::Server, Request, Response, Status};
use tracing::{error, info};
use uuid::Uuid;

pub mod gameroom {
    tonic::include_proto!("animequiz.gameroom.v1");
}

pub mod score {
    tonic::include_proto!("animequiz.score.v1");
}

use gameroom::game_room_service_server::{GameRoomService, GameRoomServiceServer};
use gameroom::*;
use score::score_service_client::ScoreServiceClient;
use score::AddScoreRequest;

#[derive(Clone)]
struct GameRoomSvc {
    db: PgPool,
    score_service_addr: String,
}

#[derive(FromRow)]
struct RoomRow {
    id: Uuid,
    name: String,
    status: String,
    created_by: Uuid,
    created_at: NaiveDateTime,
}

#[derive(FromRow)]
struct RoomPlayerRow {
    user_id: Uuid,
    username: String,
    ready: bool,
    joined_at: NaiveDateTime,
}

fn normalize_service_addr(addr: &str) -> String {
    if addr.starts_with("http://") || addr.starts_with("https://") {
        addr.to_string()
    } else {
        format!("http://{}", addr)
    }
}

impl GameRoomSvc {
    fn parse_uuid(value: &str, field: &str) -> Result<Uuid, Status> {
        if value.trim().is_empty() {
            return Err(Status::invalid_argument(format!(
                "{field} must not be empty"
            )));
        }

        Uuid::parse_str(value)
            .map_err(|_| Status::invalid_argument(format!("{field} must be a valid UUID")))
    }

    fn ensure_non_empty(value: &str, field: &str) -> Result<(), Status> {
        if value.trim().is_empty() {
            return Err(Status::invalid_argument(format!(
                "{field} must not be empty"
            )));
        }

        Ok(())
    }

    async fn get_room(&self, room_id: Uuid) -> Result<RoomRow, Status> {
        sqlx::query_as::<_, RoomRow>(
            "SELECT id, name, status, created_by, created_at FROM rooms WHERE id = $1",
        )
        .bind(room_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| Status::unavailable(format!("database error: {e}")))?
        .ok_or_else(|| Status::not_found("room not found"))
    }

    fn to_proto_room(row: RoomRow) -> Room {
        Room {
            id: row.id.to_string(),
            name: row.name,
            status: row.status,
            created_by: row.created_by.to_string(),
            created_at: row.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}

#[tonic::async_trait]
impl GameRoomService for GameRoomSvc {
    async fn create_room(
        &self,
        request: Request<CreateRoomRequest>,
    ) -> Result<Response<CreateRoomResponse>, Status> {
        let payload = request.into_inner();

        Self::ensure_non_empty(&payload.name, "name")?;
        let created_by = Self::parse_uuid(&payload.created_by, "created_by")?;

        let room_id = Uuid::new_v4();
        let now = Utc::now().naive_utc();

        sqlx::query(
            "INSERT INTO rooms (id, name, status, created_by, created_at)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(room_id)
        .bind(payload.name.trim())
        .bind("WAITING")
        .bind(created_by)
        .bind(now)
        .execute(&self.db)
        .await
        .map_err(|e| Status::unavailable(format!("database error: {e}")))?;

        info!(room_id = %room_id, created_by = %created_by, "Room created");

        let room = self.get_room(room_id).await?;

        Ok(Response::new(CreateRoomResponse {
            room: Some(Self::to_proto_room(room)),
        }))
    }

    async fn join_room(
        &self,
        request: Request<JoinRoomRequest>,
    ) -> Result<Response<JoinRoomResponse>, Status> {
        let payload = request.into_inner();

        let room_id = Self::parse_uuid(&payload.room_id, "room_id")?;
        let user_id = Self::parse_uuid(&payload.user_id, "user_id")?;

        let room = self.get_room(room_id).await?;

        if room.status == "FINISHED" {
            return Err(Status::failed_precondition("room is finished"));
        }

        let fallback_username: String = user_id.to_string().chars().take(8).collect();
        let username = if payload.username.trim().is_empty() {
            fallback_username
        } else {
            payload.username.trim().to_string()
        };

        sqlx::query(
            "INSERT INTO room_players (room_id, user_id, username, joined_at, ready)
             VALUES ($1, $2, $3, $4, true)
             ON CONFLICT (room_id, user_id)
             DO UPDATE SET
                username = CASE
                    WHEN EXCLUDED.username <> '' THEN EXCLUDED.username
                    ELSE room_players.username
                END,
                ready = true",
        )
        .bind(room_id)
        .bind(user_id)
        .bind(username)
        .bind(Utc::now().naive_utc())
        .execute(&self.db)
        .await
        .map_err(|e| Status::unavailable(format!("database error: {e}")))?;

        info!(room_id = %room_id, user_id = %user_id, "User joined room");

        Ok(Response::new(JoinRoomResponse {
            room: Some(Self::to_proto_room(room)),
        }))
    }

    async fn start_game(
        &self,
        request: Request<StartGameRequest>,
    ) -> Result<Response<StartGameResponse>, Status> {
        let room_id = Self::parse_uuid(&request.into_inner().room_id, "room_id")?;

        self.get_room(room_id).await?;

        sqlx::query("UPDATE rooms SET status = 'STARTED' WHERE id = $1")
            .bind(room_id)
            .execute(&self.db)
            .await
            .map_err(|e| Status::unavailable(format!("database error: {e}")))?;

        info!(room_id = %room_id, "Game started");

        let room = self.get_room(room_id).await?;

        Ok(Response::new(StartGameResponse {
            room: Some(Self::to_proto_room(room)),
        }))
    }

    async fn end_game(
        &self,
        request: Request<EndGameRequest>,
    ) -> Result<Response<EndGameResponse>, Status> {
        let room_id = Self::parse_uuid(&request.into_inner().room_id, "room_id")?;

        self.get_room(room_id).await?;

        sqlx::query("UPDATE rooms SET status = 'FINISHED' WHERE id = $1")
            .bind(room_id)
            .execute(&self.db)
            .await
            .map_err(|e| Status::unavailable(format!("database error: {e}")))?;

        info!(room_id = %room_id, "Game ended");

        let room = self.get_room(room_id).await?;

        Ok(Response::new(EndGameResponse {
            room: Some(Self::to_proto_room(room)),
        }))
    }

    async fn submit_answer(
        &self,
        request: Request<SubmitAnswerRequest>,
    ) -> Result<Response<SubmitAnswerResponse>, Status> {
        let payload = request.into_inner();

        let room_id = Self::parse_uuid(&payload.room_id, "room_id")?;
        let user_id = Self::parse_uuid(&payload.user_id, "user_id")?;

        Self::ensure_non_empty(&payload.selected_option, "selected_option")?;
        Self::ensure_non_empty(&payload.correct_option, "correct_option")?;
        Self::ensure_non_empty(&payload.question_id, "question_id")?;

        let room = self.get_room(room_id).await?;

        if room.status != "STARTED" {
            return Err(Status::failed_precondition("room is not started"));
        }

        let joined: bool = sqlx::query_scalar(
            "SELECT EXISTS(
                SELECT 1
                FROM room_players
                WHERE room_id = $1 AND user_id = $2
            )",
        )
        .bind(room_id)
        .bind(user_id)
        .fetch_one(&self.db)
        .await
        .map_err(|e| Status::unavailable(format!("database error: {e}")))?;

        if !joined {
            return Err(Status::failed_precondition("user is not in the room"));
        }

        let is_correct = payload.selected_option.trim() == payload.correct_option.trim();

        let inserted = sqlx::query(
            "INSERT INTO room_answers (
                room_id,
                question_id,
                user_id,
                selected_option,
                correct_option,
                is_correct
            )
            VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(room_id)
        .bind(payload.question_id.trim())
        .bind(user_id)
        .bind(payload.selected_option.trim())
        .bind(payload.correct_option.trim())
        .bind(is_correct)
        .execute(&self.db)
        .await;

        if let Err(e) = inserted {
            if let SqlxError::Database(db_err) = &e {
                if db_err.is_unique_violation() {
                    return Err(Status::failed_precondition(
                        "user already answered this question",
                    ));
                }
            }

            return Err(Status::unavailable(format!("database error: {e}")));
        }

        info!(
            room_id = %room_id,
            user_id = %user_id,
            question_id = %payload.question_id,
            correct = is_correct,
            "Answer submitted"
        );

        if !is_correct {
            return Ok(Response::new(SubmitAnswerResponse {
                correct: false,
                points_awarded: 0,
                total_points: 0,
                message: "Incorrect answer".to_string(),
            }));
        }

        let score_url = normalize_service_addr(&self.score_service_addr);

        let mut client = match ScoreServiceClient::connect(score_url.clone()).await {
            Ok(client) => client,
            Err(e) => {
                error!(
                    error = %e,
                    score_service_addr = %score_url,
                    "Failed to connect to score-service"
                );

                return Ok(Response::new(SubmitAnswerResponse {
                    correct: true,
                    points_awarded: 100,
                    total_points: 0,
                    message: "Correct answer, but score-service is unavailable".to_string(),
                }));
            }
        };

        match client
            .add_score(Request::new(AddScoreRequest {
                room_id: room_id.to_string(),
                user_id: user_id.to_string(),
                points: 100,
            }))
            .await
        {
            Ok(resp) => {
                let total_points = resp.into_inner().total_points;

                info!(
                    room_id = %room_id,
                    user_id = %user_id,
                    total_points = total_points,
                    "Score updated"
                );

                Ok(Response::new(SubmitAnswerResponse {
                    correct: true,
                    points_awarded: 100,
                    total_points,
                    message: "Correct answer".to_string(),
                }))
            }
            Err(e) => {
                error!(error = %e, "Score-service failed on AddScore");

                Ok(Response::new(SubmitAnswerResponse {
                    correct: true,
                    points_awarded: 100,
                    total_points: 0,
                    message: "Correct answer, but score could not be updated".to_string(),
                }))
            }
        }
    }

    async fn get_room_state(
        &self,
        request: Request<GetRoomStateRequest>,
    ) -> Result<Response<GetRoomStateResponse>, Status> {
        let payload = request.into_inner();

        let room_id = Self::parse_uuid(&payload.room_id, "room_id")?;
        let room = self.get_room(room_id).await?;

        let current_q = payload.current_question_id.trim().to_string();
        let has_question = !current_q.is_empty();

        let player_rows = sqlx::query_as::<_, RoomPlayerRow>(
            "SELECT user_id, username, ready, joined_at
             FROM room_players
             WHERE room_id = $1
             ORDER BY joined_at ASC",
        )
        .bind(room_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| Status::unavailable(format!("database error: {e}")))?;

        let total_players = player_rows.len() as i32;

        let answered_count = if has_question {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*)
                 FROM room_answers
                 WHERE room_id = $1 AND question_id = $2",
            )
            .bind(room_id)
            .bind(&current_q)
            .fetch_one(&self.db)
            .await
            .map_err(|e| Status::unavailable(format!("database error: {e}")))?
                as i32
        } else {
            0
        };

        let mut players = Vec::with_capacity(player_rows.len());

        for player in player_rows {
            let answered_current_question = if has_question {
                sqlx::query_scalar::<_, bool>(
                    "SELECT EXISTS(
                        SELECT 1
                        FROM room_answers
                        WHERE room_id = $1 AND question_id = $2 AND user_id = $3
                    )",
                )
                .bind(room_id)
                .bind(&current_q)
                .bind(player.user_id)
                .fetch_one(&self.db)
                .await
                .map_err(|e| Status::unavailable(format!("database error: {e}")))?
            } else {
                false
            };

            let fallback_username: String = player.user_id.to_string().chars().take(8).collect();

            let username = if player.username.trim().is_empty() {
                fallback_username
            } else {
                player.username
            };

            players.push(RoomPlayer {
                user_id: player.user_id.to_string(),
                username,
                answered_current_question,
                ready: player.ready,
                joined_at: player.joined_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            });
        }

        let all_answered = has_question && total_players > 0 && answered_count == total_players;

        Ok(Response::new(GetRoomStateResponse {
            room: Some(Self::to_proto_room(room)),
            players,
            total_players,
            answered_count,
            all_answered,
        }))
    }
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "game-room-service"
    }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@rooms-db:5432/rooms_db".to_string());

    let score_service_addr =
        env::var("SCORE_SERVICE_ADDR").unwrap_or_else(|_| "score-service:50054".to_string());

    let grpc_port = env::var("GRPC_PORT")
        .or_else(|_| env::var("GAME_ROOM_GRPC_PORT"))
        .unwrap_or_else(|_| "50053".to_string());

    let grpc_addr: SocketAddr = format!("0.0.0.0:{grpc_port}").parse()?;

    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    info!("Connected to PostgreSQL");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS rooms (
            id UUID PRIMARY KEY,
            name VARCHAR NOT NULL,
            status VARCHAR NOT NULL,
            created_by UUID NOT NULL,
            created_at TIMESTAMP NOT NULL
        )",
    )
    .execute(&db)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS room_players (
            room_id UUID NOT NULL,
            user_id UUID NOT NULL,
            username TEXT NOT NULL DEFAULT '',
            joined_at TIMESTAMP NOT NULL DEFAULT NOW(),
            ready BOOLEAN NOT NULL DEFAULT true,
            PRIMARY KEY (room_id, user_id)
        )",
    )
    .execute(&db)
    .await?;

    sqlx::query(
        "ALTER TABLE room_players
         ADD COLUMN IF NOT EXISTS username TEXT NOT NULL DEFAULT ''",
    )
    .execute(&db)
    .await?;

    sqlx::query(
        "ALTER TABLE room_players
         ADD COLUMN IF NOT EXISTS joined_at TIMESTAMP NOT NULL DEFAULT NOW()",
    )
    .execute(&db)
    .await?;

    sqlx::query(
        "ALTER TABLE room_players
         ADD COLUMN IF NOT EXISTS ready BOOLEAN NOT NULL DEFAULT true",
    )
    .execute(&db)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS room_answers (
            room_id UUID NOT NULL,
            question_id TEXT NOT NULL,
            user_id UUID NOT NULL,
            selected_option TEXT NOT NULL,
            correct_option TEXT NOT NULL,
            is_correct BOOLEAN NOT NULL,
            answered_at TIMESTAMP NOT NULL DEFAULT NOW(),
            PRIMARY KEY (room_id, question_id, user_id)
        )",
    )
    .execute(&db)
    .await?;

    info!("Game room tables ready");

    let service = GameRoomSvc {
        db,
        score_service_addr: score_service_addr.clone(),
    };

    info!("Score service addr: {}", normalize_service_addr(&score_service_addr));
    info!("Starting game-room-service gRPC on {}", grpc_addr);

    let grpc_server = async move {
        Server::builder()
            .add_service(GameRoomServiceServer::new(service))
            .serve(grpc_addr)
            .await
            .map_err(anyhow::Error::from)
    };

    let port = env::var("PORT").unwrap_or_else(|_| "10000".to_string());
    let http_addr: SocketAddr = format!("0.0.0.0:{port}").parse()?;

    let app = Router::new().route("/health", get(health));

    info!("Starting game-room-service HTTP health on {}", http_addr);

    let http_server = async move {
        let listener = TcpListener::bind(http_addr).await?;
        axum::serve(listener, app).await?;
        Ok::<(), anyhow::Error>(())
    };

    tokio::try_join!(grpc_server, http_server)?;

    Ok(())
}