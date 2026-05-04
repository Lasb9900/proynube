use chrono::{NaiveDateTime, Utc};
use sqlx::{postgres::PgPoolOptions, FromRow, PgPool};
use std::env;
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
            "INSERT INTO rooms (id, name, status, created_by, created_at) VALUES ($1, $2, $3, $4, $5)",
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

        sqlx::query(
            "INSERT INTO room_players (room_id, user_id, joined_at) VALUES ($1, $2, $3) ON CONFLICT (room_id, user_id) DO NOTHING",
        )
        .bind(room_id)
        .bind(user_id)
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
        info!(room_id = %room_id, user_id = %user_id, question_id = %payload.question_id, correct = is_correct, "Answer submitted");

        if !is_correct {
            return Ok(Response::new(SubmitAnswerResponse {
                correct: false,
                points_awarded: 0,
                total_points: 0,
                message: "Incorrect answer".to_string(),
            }));
        }

        let mut client = match ScoreServiceClient::connect(format!(
            "http://{}",
            self.score_service_addr
        ))
        .await
        {
            Ok(client) => client,
            Err(e) => {
                error!(error = %e, "Failed to connect to score-service");
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
                info!(room_id = %room_id, user_id = %user_id, total_points = total_points, "Score updated");
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@rooms-db:5432/rooms_db".to_string());
    let score_service_addr =
        env::var("SCORE_SERVICE_ADDR").unwrap_or_else(|_| "score-service:50054".to_string());

    info!("Starting game-room-service on 0.0.0.0:50053");

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
            joined_at TIMESTAMP NOT NULL,
            PRIMARY KEY (room_id, user_id)
        )",
    )
    .execute(&db)
    .await?;
    info!("Game room tables ready");

    let addr = "0.0.0.0:50053".parse()?;
    let service = GameRoomSvc {
        db,
        score_service_addr,
    };

    Server::builder()
        .add_service(GameRoomServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
