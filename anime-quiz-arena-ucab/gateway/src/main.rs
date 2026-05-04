use std::env;

use tonic::{transport::Channel, transport::Server, Request, Response, Status};
use tracing::{error, info};

pub mod gateway {
    tonic::include_proto!("animequiz.gateway.v1");
}
pub mod users {
    tonic::include_proto!("animequiz.users.v1");
}
pub mod questions {
    tonic::include_proto!("animequiz.questions.v1");
}
pub mod gameroom {
    tonic::include_proto!("animequiz.gameroom.v1");
}
pub mod score {
    tonic::include_proto!("animequiz.score.v1");
}

use gateway::gateway_service_server::{GatewayService, GatewayServiceServer};
use gateway::*;

#[derive(Clone)]
struct GatewayServerImpl {
    users_addr: String,
    questions_addr: String,
    game_room_addr: String,
    score_addr: String,
}

impl GatewayServerImpl {
    async fn users_client(&self) -> Result<users::users_service_client::UsersServiceClient<Channel>, Status> {
        let target = format!("http://{}", self.users_addr);
        info!(target = %target, "Conectando a users-service");
        users::users_service_client::UsersServiceClient::connect(target)
            .await
            .map_err(|e| {
                error!(error = %e, "users-service no disponible");
                Status::unavailable("users-service no disponible")
            })
    }

    async fn questions_client(&self) -> Result<questions::questions_service_client::QuestionsServiceClient<Channel>, Status> {
        let target = format!("http://{}", self.questions_addr);
        info!(target = %target, "Conectando a questions-service");
        questions::questions_service_client::QuestionsServiceClient::connect(target)
            .await
            .map_err(|e| {
                error!(error = %e, "questions-service no disponible");
                Status::unavailable("questions-service no disponible")
            })
    }

    async fn game_room_client(&self) -> Result<gameroom::game_room_service_client::GameRoomServiceClient<Channel>, Status> {
        let target = format!("http://{}", self.game_room_addr);
        info!(target = %target, "Conectando a game-room-service");
        gameroom::game_room_service_client::GameRoomServiceClient::connect(target)
            .await
            .map_err(|e| {
                error!(error = %e, "game-room-service no disponible");
                Status::unavailable("game-room-service no disponible")
            })
    }

    async fn score_client(&self) -> Result<score::score_service_client::ScoreServiceClient<Channel>, Status> {
        let target = format!("http://{}", self.score_addr);
        info!(target = %target, "Conectando a score-service");
        score::score_service_client::ScoreServiceClient::connect(target)
            .await
            .map_err(|e| {
                error!(error = %e, "score-service no disponible");
                Status::unavailable("score-service no disponible")
            })
    }
}

#[tonic::async_trait]
impl GatewayService for GatewayServerImpl {
    async fn create_user(
        &self,
        request: Request<GatewayCreateUserRequest>,
    ) -> Result<Response<GatewayUserResponse>, Status> {
        info!("Gateway/CreateUser recibido");
        let payload = request.into_inner();
        let mut client = self.users_client().await?;
        let response = client
            .create_user(users::CreateUserRequest {
                username: payload.username,
                email: payload.email,
                password: payload.password,
            })
            .await
            .map_err(|e| {
                error!(error = %e, "Error en users-service CreateUser");
                Status::new(e.code(), e.message().to_string())
            })?
            .into_inner();

        Ok(Response::new(GatewayUserResponse {
            user: response.user.map(|u| GatewayUser {
                id: u.id,
                username: u.username,
                email: u.email,
                created_at: u.created_at,
            }),
        }))
    }

    async fn login_basic(&self, request: Request<GatewayLoginRequest>) -> Result<Response<GatewayLoginResponse>, Status> {
        info!("Gateway/LoginBasic recibido");
        let payload = request.into_inner();
        let mut client = self.users_client().await?;
        let response = client
            .login_basic(users::LoginBasicRequest {
                email: payload.email,
                password: payload.password,
            })
            .await
            .map_err(|e| {
                error!(error = %e, "Error en users-service LoginBasic");
                Status::new(e.code(), e.message().to_string())
            })?
            .into_inner();

        Ok(Response::new(GatewayLoginResponse {
            success: response.success,
            user: response.user.map(|u| GatewayUser {
                id: u.id,
                username: u.username,
                email: u.email,
                created_at: u.created_at,
            }),
        }))
    }

    async fn search_anime(&self, request: Request<GatewaySearchAnimeRequest>) -> Result<Response<GatewaySearchAnimeResponse>, Status> {
        info!("Gateway/SearchAnime recibido");
        let payload = request.into_inner();
        let mut client = self.questions_client().await?;
        let response = client
            .search_anime(questions::SearchAnimeRequest { query: payload.query })
            .await
            .map_err(|e| {
                error!(error = %e, "Error en questions-service SearchAnime");
                Status::new(e.code(), e.message().to_string())
            })?
            .into_inner();

        Ok(Response::new(GatewaySearchAnimeResponse {
            animes: response
                .animes
                .into_iter()
                .map(|a| GatewayAnime {
                    id: a.id,
                    title: a.title,
                    synopsis: a.synopsis,
                    episodes: a.episodes,
                    score: a.score,
                    image_url: a.image_url,
                })
                .collect(),
        }))
    }

    async fn generate_question(&self, request: Request<GatewayGenerateQuestionRequest>) -> Result<Response<GatewayGenerateQuestionResponse>, Status> {
        info!("Gateway/GenerateQuestion recibido");
        let payload = request.into_inner();
        let mut client = self.questions_client().await?;
        let response = client
            .generate_question(questions::GenerateQuestionRequest { room_id: payload.room_id })
            .await
            .map_err(|e| {
                error!(error = %e, "Error en questions-service GenerateQuestion");
                Status::new(e.code(), e.message().to_string())
            })?
            .into_inner();

        Ok(Response::new(GatewayGenerateQuestionResponse {
            question: response.question.map(|q| GatewayQuestion {
                id: q.id,
                text: q.text,
                option_a: q.option_a,
                option_b: q.option_b,
                option_c: q.option_c,
                option_d: q.option_d,
                correct_option: q.correct_option,
                anime_id: q.anime_id,
            }),
        }))
    }

    async fn create_room(&self, request: Request<GatewayCreateRoomRequest>) -> Result<Response<GatewayRoomResponse>, Status> {
        info!("Gateway/CreateRoom recibido");
        let payload = request.into_inner();
        let mut client = self.game_room_client().await?;
        let response = client
            .create_room(gameroom::CreateRoomRequest {
                name: payload.name,
                created_by: payload.created_by,
            })
            .await
            .map_err(|e| {
                error!(error = %e, "Error en game-room-service CreateRoom");
                Status::new(e.code(), e.message().to_string())
            })?
            .into_inner();

        Ok(Response::new(GatewayRoomResponse {
            room: response.room.map(map_room),
        }))
    }

    async fn join_room(&self, request: Request<GatewayJoinRoomRequest>) -> Result<Response<GatewayRoomResponse>, Status> {
        info!("Gateway/JoinRoom recibido");
        let payload = request.into_inner();
        let mut client = self.game_room_client().await?;
        let response = client
            .join_room(gameroom::JoinRoomRequest {
                room_id: payload.room_id,
                user_id: payload.user_id,
            })
            .await
            .map_err(|e| {
                error!(error = %e, "Error en game-room-service JoinRoom");
                Status::new(e.code(), e.message().to_string())
            })?
            .into_inner();

        Ok(Response::new(GatewayRoomResponse { room: response.room.map(map_room) }))
    }

    async fn start_game(&self, request: Request<GatewayRoomIdRequest>) -> Result<Response<GatewayRoomResponse>, Status> {
        info!("Gateway/StartGame recibido");
        let payload = request.into_inner();
        let mut client = self.game_room_client().await?;
        let response = client
            .start_game(gameroom::StartGameRequest { room_id: payload.room_id })
            .await
            .map_err(|e| {
                error!(error = %e, "Error en game-room-service StartGame");
                Status::new(e.code(), e.message().to_string())
            })?
            .into_inner();

        Ok(Response::new(GatewayRoomResponse { room: response.room.map(map_room) }))
    }

    async fn end_game(&self, request: Request<GatewayRoomIdRequest>) -> Result<Response<GatewayRoomResponse>, Status> {
        info!("Gateway/EndGame recibido");
        let payload = request.into_inner();
        let mut client = self.game_room_client().await?;
        let response = client
            .end_game(gameroom::EndGameRequest { room_id: payload.room_id })
            .await
            .map_err(|e| {
                error!(error = %e, "Error en game-room-service EndGame");
                Status::new(e.code(), e.message().to_string())
            })?
            .into_inner();

        Ok(Response::new(GatewayRoomResponse { room: response.room.map(map_room) }))
    }

    async fn submit_answer(
        &self,
        request: Request<GatewaySubmitAnswerRequest>,
    ) -> Result<Response<GatewaySubmitAnswerResponse>, Status> {
        info!("Gateway/SubmitAnswer recibido");
        let payload = request.into_inner();
        let mut client = self.game_room_client().await?;
        let response = client
            .submit_answer(gameroom::SubmitAnswerRequest {
                room_id: payload.room_id,
                user_id: payload.user_id,
                question_id: payload.question_id,
                selected_option: payload.selected_option,
                correct_option: payload.correct_option,
            })
            .await
            .map_err(|e| {
                error!(error = %e, "Error en game-room-service SubmitAnswer");
                Status::new(e.code(), e.message().to_string())
            })?
            .into_inner();

        Ok(Response::new(GatewaySubmitAnswerResponse {
            correct: response.correct,
            points_awarded: response.points_awarded,
            total_points: response.total_points,
            message: response.message,
        }))
    }

    async fn get_leaderboard(
        &self,
        request: Request<GatewayLeaderboardRequest>,
    ) -> Result<Response<GatewayLeaderboardResponse>, Status> {
        info!("Gateway/GetLeaderboard recibido");
        let payload = request.into_inner();
        let mut client = self.score_client().await?;
        let response = client
            .get_leaderboard(score::GetLeaderboardRequest {
                room_id: payload.room_id,
                limit: payload.limit,
            })
            .await
            .map_err(|e| {
                error!(error = %e, "Error en score-service GetLeaderboard");
                Status::new(e.code(), e.message().to_string())
            })?
            .into_inner();

        Ok(Response::new(GatewayLeaderboardResponse {
            entries: response
                .entries
                .into_iter()
                .map(|e| GatewayScoreEntry {
                    user_id: e.user_id,
                    points: e.points,
                    rank: e.rank,
                })
                .collect(),
        }))
    }
}

fn map_room(room: gameroom::Room) -> GatewayRoom {
    GatewayRoom {
        id: room.id,
        name: room.name,
        status: room.status,
        created_by: room.created_by,
        created_at: room.created_at,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let addr = "0.0.0.0:50050".parse()?;
    let gateway = GatewayServerImpl {
        users_addr: env::var("USERS_SERVICE_ADDR").unwrap_or_else(|_| "users-service:50051".to_string()),
        questions_addr: env::var("QUESTIONS_SERVICE_ADDR").unwrap_or_else(|_| "questions-service:50052".to_string()),
        game_room_addr: env::var("GAME_ROOM_SERVICE_ADDR").unwrap_or_else(|_| "game-room-service:50053".to_string()),
        score_addr: env::var("SCORE_SERVICE_ADDR").unwrap_or_else(|_| "score-service:50054".to_string()),
    };

    info!("Starting api-gateway on 0.0.0.0:50050");
    Server::builder()
        .add_service(GatewayServiceServer::new(gateway))
        .serve(addr)
        .await?;

    Ok(())
}
