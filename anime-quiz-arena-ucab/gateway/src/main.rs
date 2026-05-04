use std::{env, net::SocketAddr};

use anyhow::anyhow;
use axum::{
    extract::{Path, Query, State},
    http::{Method, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tonic::{transport::Channel, transport::Server, Code, Request, Response, Status};
use tower_http::cors::CorsLayer;
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
struct AppState {
    users_addr: String,
    questions_addr: String,
    game_room_addr: String,
    score_addr: String,
}

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<ApiErrorResponse>)>;

#[derive(Debug, Serialize)]
struct ApiErrorResponse {
    error: String,
    code: String,
}
#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}
#[derive(Debug, Serialize)]
struct UserDto {
    id: String,
    username: String,
    email: String,
    created_at: String,
}
#[derive(Debug, Deserialize)]
struct CreateUserHttpRequest {
    username: String,
    email: String,
    password: String,
}
#[derive(Debug, Deserialize)]
struct LoginHttpRequest {
    email: String,
    password: String,
}
#[derive(Debug, Serialize)]
struct LoginHttpResponse {
    success: bool,
    user: Option<UserDto>,
}
#[derive(Debug, Serialize)]
struct AnimeDto {
    id: i32,
    title: String,
    synopsis: String,
    episodes: i32,
    score: f64,
    image_url: String,
}
#[derive(Debug, Serialize)]
struct SearchAnimeHttpResponse {
    animes: Vec<AnimeDto>,
}
#[derive(Debug, Serialize)]
struct QuestionDto {
    id: String,
    text: String,
    option_a: String,
    option_b: String,
    option_c: String,
    option_d: String,
    correct_option: String,
    anime_id: i32,
}
#[derive(Debug, Deserialize)]
struct GenerateQuestionHttpRequest {
    room_id: String,
}
#[derive(Debug, Serialize)]
struct GenerateQuestionHttpResponse {
    question: Option<QuestionDto>,
}
#[derive(Debug, Serialize)]
struct RoomDto {
    id: String,
    name: String,
    status: String,
    created_by: String,
    created_at: String,
}
#[derive(Debug, Deserialize)]
struct CreateRoomHttpRequest {
    name: String,
    created_by: String,
}
#[derive(Debug, Deserialize)]
struct JoinRoomHttpRequest {
    user_id: String,
}
#[derive(Debug, Serialize)]
struct RoomHttpResponse {
    room: Option<RoomDto>,
}
#[derive(Debug, Deserialize)]
struct SubmitAnswerHttpRequest {
    user_id: String,
    question_id: String,
    selected_option: String,
    correct_option: String,
}
#[derive(Debug, Serialize)]
struct SubmitAnswerHttpResponse {
    correct: bool,
    points_awarded: i32,
    total_points: i32,
    message: String,
}
#[derive(Debug, Serialize)]
struct ScoreEntryDto {
    user_id: String,
    points: i32,
    rank: i32,
}
#[derive(Debug, Serialize)]
struct LeaderboardHttpResponse {
    entries: Vec<ScoreEntryDto>,
}
#[derive(Debug, Deserialize)]
struct SearchAnimeQuery {
    q: String,
}
#[derive(Debug, Deserialize)]
struct LeaderboardQuery {
    limit: Option<i32>,
}

impl From<users::User> for UserDto {
    fn from(u: users::User) -> Self {
        Self {
            id: u.id,
            username: u.username,
            email: u.email,
            created_at: u.created_at,
        }
    }
}
impl From<questions::Anime> for AnimeDto {
    fn from(a: questions::Anime) -> Self {
        Self {
            id: a.id,
            title: a.title,
            synopsis: a.synopsis,
            episodes: a.episodes,
            score: a.score,
            image_url: a.image_url,
        }
    }
}
impl From<questions::Question> for QuestionDto {
    fn from(q: questions::Question) -> Self {
        Self {
            id: q.id,
            text: q.text,
            option_a: q.option_a,
            option_b: q.option_b,
            option_c: q.option_c,
            option_d: q.option_d,
            correct_option: q.correct_option,
            anime_id: q.anime_id,
        }
    }
}
impl From<gameroom::Room> for RoomDto {
    fn from(r: gameroom::Room) -> Self {
        Self {
            id: r.id,
            name: r.name,
            status: r.status,
            created_by: r.created_by,
            created_at: r.created_at,
        }
    }
}
impl From<score::ScoreEntry> for ScoreEntryDto {
    fn from(s: score::ScoreEntry) -> Self {
        Self {
            user_id: s.user_id,
            points: s.points,
            rank: s.rank,
        }
    }
}

fn grpc_status_to_http(status: tonic::Status) -> (StatusCode, Json<ApiErrorResponse>) {
    let (http_status, code) = match status.code() {
        Code::InvalidArgument => (StatusCode::BAD_REQUEST, "invalid_argument"),
        Code::Unauthenticated => (StatusCode::UNAUTHORIZED, "unauthenticated"),
        Code::NotFound => (StatusCode::NOT_FOUND, "not_found"),
        Code::AlreadyExists => (StatusCode::CONFLICT, "already_exists"),
        Code::FailedPrecondition => (StatusCode::PRECONDITION_FAILED, "failed_precondition"),
        Code::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
    };
    (
        http_status,
        Json(ApiErrorResponse {
            error: status.message().to_string(),
            code: code.to_string(),
        }),
    )
}

fn internal_connect_error() -> (StatusCode, Json<ApiErrorResponse>) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ApiErrorResponse {
            error: "No se pudo conectar al servicio interno".to_string(),
            code: "unavailable".to_string(),
        }),
    )
}

async fn users_client(
    state: &AppState,
) -> Result<
    users::users_service_client::UsersServiceClient<Channel>,
    (StatusCode, Json<ApiErrorResponse>),
> {
    users::users_service_client::UsersServiceClient::connect(format!("http://{}", state.users_addr))
        .await
        .map_err(|_| internal_connect_error())
}
async fn questions_client(
    state: &AppState,
) -> Result<
    questions::questions_service_client::QuestionsServiceClient<Channel>,
    (StatusCode, Json<ApiErrorResponse>),
> {
    questions::questions_service_client::QuestionsServiceClient::connect(format!(
        "http://{}",
        state.questions_addr
    ))
    .await
    .map_err(|_| internal_connect_error())
}
async fn game_room_client(
    state: &AppState,
) -> Result<
    gameroom::game_room_service_client::GameRoomServiceClient<Channel>,
    (StatusCode, Json<ApiErrorResponse>),
> {
    gameroom::game_room_service_client::GameRoomServiceClient::connect(format!(
        "http://{}",
        state.game_room_addr
    ))
    .await
    .map_err(|_| internal_connect_error())
}
async fn score_client(
    state: &AppState,
) -> Result<
    score::score_service_client::ScoreServiceClient<Channel>,
    (StatusCode, Json<ApiErrorResponse>),
> {
    score::score_service_client::ScoreServiceClient::connect(format!("http://{}", state.score_addr))
        .await
        .map_err(|_| internal_connect_error())
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "api-gateway",
    })
}
async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserHttpRequest>,
) -> ApiResult<UserDto> {
    let mut client = users_client(&state).await?;
    let res = client
        .create_user(users::CreateUserRequest {
            username: payload.username,
            email: payload.email,
            password: payload.password,
        })
        .await
        .map_err(grpc_status_to_http)?
        .into_inner();
    let user = res
        .user
        .ok_or_else(|| grpc_status_to_http(Status::internal("Respuesta sin usuario")))?;
    Ok(Json(UserDto::from(user)))
}
async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginHttpRequest>,
) -> ApiResult<LoginHttpResponse> {
    let mut client = users_client(&state).await?;
    let res = client
        .login_basic(users::LoginBasicRequest {
            email: payload.email,
            password: payload.password,
        })
        .await
        .map_err(grpc_status_to_http)?
        .into_inner();
    Ok(Json(LoginHttpResponse {
        success: res.success,
        user: res.user.map(UserDto::from),
    }))
}
async fn generate_question(
    State(state): State<AppState>,
    Json(payload): Json<GenerateQuestionHttpRequest>,
) -> ApiResult<GenerateQuestionHttpResponse> {
    let mut client = questions_client(&state).await?;
    let res = client
        .generate_question(questions::GenerateQuestionRequest {
            room_id: payload.room_id,
        })
        .await
        .map_err(grpc_status_to_http)?
        .into_inner();
    Ok(Json(GenerateQuestionHttpResponse {
        question: res.question.map(QuestionDto::from),
    }))
}
async fn search_anime(
    State(state): State<AppState>,
    Query(query): Query<SearchAnimeQuery>,
) -> ApiResult<SearchAnimeHttpResponse> {
    let mut client = questions_client(&state).await?;
    let res = client
        .search_anime(questions::SearchAnimeRequest { query: query.q })
        .await
        .map_err(grpc_status_to_http)?
        .into_inner();
    Ok(Json(SearchAnimeHttpResponse {
        animes: res.animes.into_iter().map(AnimeDto::from).collect(),
    }))
}
async fn create_room(
    State(state): State<AppState>,
    Json(payload): Json<CreateRoomHttpRequest>,
) -> ApiResult<RoomHttpResponse> {
    let mut client = game_room_client(&state).await?;
    let res = client
        .create_room(gameroom::CreateRoomRequest {
            name: payload.name,
            created_by: payload.created_by,
        })
        .await
        .map_err(grpc_status_to_http)?
        .into_inner();
    Ok(Json(RoomHttpResponse {
        room: res.room.map(RoomDto::from),
    }))
}
async fn join_room(
    Path(room_id): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<JoinRoomHttpRequest>,
) -> ApiResult<RoomHttpResponse> {
    let mut client = game_room_client(&state).await?;
    let res = client
        .join_room(gameroom::JoinRoomRequest {
            room_id,
            user_id: payload.user_id,
        })
        .await
        .map_err(grpc_status_to_http)?
        .into_inner();
    Ok(Json(RoomHttpResponse {
        room: res.room.map(RoomDto::from),
    }))
}
async fn start_game(
    Path(room_id): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<RoomHttpResponse> {
    let mut client = game_room_client(&state).await?;
    let res = client
        .start_game(gameroom::StartGameRequest { room_id })
        .await
        .map_err(grpc_status_to_http)?
        .into_inner();
    Ok(Json(RoomHttpResponse {
        room: res.room.map(RoomDto::from),
    }))
}
async fn submit_answer(
    Path(room_id): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<SubmitAnswerHttpRequest>,
) -> ApiResult<SubmitAnswerHttpResponse> {
    let mut client = game_room_client(&state).await?;
    let res = client
        .submit_answer(gameroom::SubmitAnswerRequest {
            room_id,
            user_id: payload.user_id,
            question_id: payload.question_id,
            selected_option: payload.selected_option,
            correct_option: payload.correct_option,
        })
        .await
        .map_err(grpc_status_to_http)?
        .into_inner();
    Ok(Json(SubmitAnswerHttpResponse {
        correct: res.correct,
        points_awarded: res.points_awarded,
        total_points: res.total_points,
        message: res.message,
    }))
}
async fn get_leaderboard(
    Path(room_id): Path<String>,
    State(state): State<AppState>,
    Query(query): Query<LeaderboardQuery>,
) -> ApiResult<LeaderboardHttpResponse> {
    let mut client = score_client(&state).await?;
    let res = client
        .get_leaderboard(score::GetLeaderboardRequest {
            room_id,
            limit: query.limit.unwrap_or(10),
        })
        .await
        .map_err(grpc_status_to_http)?
        .into_inner();
    Ok(Json(LeaderboardHttpResponse {
        entries: res.entries.into_iter().map(ScoreEntryDto::from).collect(),
    }))
}
async fn end_game(
    Path(room_id): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<RoomHttpResponse> {
    let mut client = game_room_client(&state).await?;
    let res = client
        .end_game(gameroom::EndGameRequest { room_id })
        .await
        .map_err(grpc_status_to_http)?
        .into_inner();
    Ok(Json(RoomHttpResponse {
        room: res.room.map(RoomDto::from),
    }))
}

#[derive(Clone)]
struct GatewayServerImpl {
    users_addr: String,
    questions_addr: String,
    game_room_addr: String,
    score_addr: String,
}
impl GatewayServerImpl {
    async fn users_client(
        &self,
    ) -> Result<users::users_service_client::UsersServiceClient<Channel>, Status> {
        users::users_service_client::UsersServiceClient::connect(format!(
            "http://{}",
            self.users_addr
        ))
        .await
        .map_err(|e| {
            error!(error = %e, "users-service no disponible");
            Status::unavailable("users-service no disponible")
        })
    }
    async fn questions_client(
        &self,
    ) -> Result<questions::questions_service_client::QuestionsServiceClient<Channel>, Status> {
        questions::questions_service_client::QuestionsServiceClient::connect(format!(
            "http://{}",
            self.questions_addr
        ))
        .await
        .map_err(|e| {
            error!(error = %e, "questions-service no disponible");
            Status::unavailable("questions-service no disponible")
        })
    }
    async fn game_room_client(
        &self,
    ) -> Result<gameroom::game_room_service_client::GameRoomServiceClient<Channel>, Status> {
        gameroom::game_room_service_client::GameRoomServiceClient::connect(format!(
            "http://{}",
            self.game_room_addr
        ))
        .await
        .map_err(|e| {
            error!(error = %e, "game-room-service no disponible");
            Status::unavailable("game-room-service no disponible")
        })
    }
    async fn score_client(
        &self,
    ) -> Result<score::score_service_client::ScoreServiceClient<Channel>, Status> {
        score::score_service_client::ScoreServiceClient::connect(format!(
            "http://{}",
            self.score_addr
        ))
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
        let p = request.into_inner();
        let mut c = self.users_client().await?;
        let r = c
            .create_user(users::CreateUserRequest {
                username: p.username,
                email: p.email,
                password: p.password,
            })
            .await
            .map_err(|e| Status::new(e.code(), e.message().to_string()))?
            .into_inner();
        Ok(Response::new(GatewayUserResponse {
            user: r.user.map(|u| GatewayUser {
                id: u.id,
                username: u.username,
                email: u.email,
                created_at: u.created_at,
            }),
        }))
    }
    async fn login_basic(
        &self,
        request: Request<GatewayLoginRequest>,
    ) -> Result<Response<GatewayLoginResponse>, Status> {
        let p = request.into_inner();
        let mut c = self.users_client().await?;
        let r = c
            .login_basic(users::LoginBasicRequest {
                email: p.email,
                password: p.password,
            })
            .await
            .map_err(|e| Status::new(e.code(), e.message().to_string()))?
            .into_inner();
        Ok(Response::new(GatewayLoginResponse {
            success: r.success,
            user: r.user.map(|u| GatewayUser {
                id: u.id,
                username: u.username,
                email: u.email,
                created_at: u.created_at,
            }),
        }))
    }
    async fn search_anime(
        &self,
        request: Request<GatewaySearchAnimeRequest>,
    ) -> Result<Response<GatewaySearchAnimeResponse>, Status> {
        let p = request.into_inner();
        let mut c = self.questions_client().await?;
        let r = c
            .search_anime(questions::SearchAnimeRequest { query: p.query })
            .await
            .map_err(|e| Status::new(e.code(), e.message().to_string()))?
            .into_inner();
        Ok(Response::new(GatewaySearchAnimeResponse {
            animes: r
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
    async fn generate_question(
        &self,
        request: Request<GatewayGenerateQuestionRequest>,
    ) -> Result<Response<GatewayGenerateQuestionResponse>, Status> {
        let p = request.into_inner();
        let mut c = self.questions_client().await?;
        let r = c
            .generate_question(questions::GenerateQuestionRequest { room_id: p.room_id })
            .await
            .map_err(|e| Status::new(e.code(), e.message().to_string()))?
            .into_inner();
        Ok(Response::new(GatewayGenerateQuestionResponse {
            question: r.question.map(|q| GatewayQuestion {
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
    async fn create_room(
        &self,
        request: Request<GatewayCreateRoomRequest>,
    ) -> Result<Response<GatewayRoomResponse>, Status> {
        let p = request.into_inner();
        let mut c = self.game_room_client().await?;
        let r = c
            .create_room(gameroom::CreateRoomRequest {
                name: p.name,
                created_by: p.created_by,
            })
            .await
            .map_err(|e| Status::new(e.code(), e.message().to_string()))?
            .into_inner();
        Ok(Response::new(GatewayRoomResponse {
            room: r.room.map(map_room),
        }))
    }
    async fn join_room(
        &self,
        request: Request<GatewayJoinRoomRequest>,
    ) -> Result<Response<GatewayRoomResponse>, Status> {
        let p = request.into_inner();
        let mut c = self.game_room_client().await?;
        let r = c
            .join_room(gameroom::JoinRoomRequest {
                room_id: p.room_id,
                user_id: p.user_id,
            })
            .await
            .map_err(|e| Status::new(e.code(), e.message().to_string()))?
            .into_inner();
        Ok(Response::new(GatewayRoomResponse {
            room: r.room.map(map_room),
        }))
    }
    async fn start_game(
        &self,
        request: Request<GatewayRoomIdRequest>,
    ) -> Result<Response<GatewayRoomResponse>, Status> {
        let p = request.into_inner();
        let mut c = self.game_room_client().await?;
        let r = c
            .start_game(gameroom::StartGameRequest { room_id: p.room_id })
            .await
            .map_err(|e| Status::new(e.code(), e.message().to_string()))?
            .into_inner();
        Ok(Response::new(GatewayRoomResponse {
            room: r.room.map(map_room),
        }))
    }
    async fn end_game(
        &self,
        request: Request<GatewayRoomIdRequest>,
    ) -> Result<Response<GatewayRoomResponse>, Status> {
        let p = request.into_inner();
        let mut c = self.game_room_client().await?;
        let r = c
            .end_game(gameroom::EndGameRequest { room_id: p.room_id })
            .await
            .map_err(|e| Status::new(e.code(), e.message().to_string()))?
            .into_inner();
        Ok(Response::new(GatewayRoomResponse {
            room: r.room.map(map_room),
        }))
    }
    async fn submit_answer(
        &self,
        request: Request<GatewaySubmitAnswerRequest>,
    ) -> Result<Response<GatewaySubmitAnswerResponse>, Status> {
        let p = request.into_inner();
        let mut c = self.game_room_client().await?;
        let r = c
            .submit_answer(gameroom::SubmitAnswerRequest {
                room_id: p.room_id,
                user_id: p.user_id,
                question_id: p.question_id,
                selected_option: p.selected_option,
                correct_option: p.correct_option,
            })
            .await
            .map_err(|e| Status::new(e.code(), e.message().to_string()))?
            .into_inner();
        Ok(Response::new(GatewaySubmitAnswerResponse {
            correct: r.correct,
            points_awarded: r.points_awarded,
            total_points: r.total_points,
            message: r.message,
        }))
    }
    async fn get_leaderboard(
        &self,
        request: Request<GatewayLeaderboardRequest>,
    ) -> Result<Response<GatewayLeaderboardResponse>, Status> {
        let p = request.into_inner();
        let mut c = self.score_client().await?;
        let r = c
            .get_leaderboard(score::GetLeaderboardRequest {
                room_id: p.room_id,
                limit: p.limit,
            })
            .await
            .map_err(|e| Status::new(e.code(), e.message().to_string()))?
            .into_inner();
        Ok(Response::new(GatewayLeaderboardResponse {
            entries: r
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

    let state = AppState {
        users_addr: env::var("USERS_SERVICE_ADDR")
            .unwrap_or_else(|_| "users-service:50051".to_string()),
        questions_addr: env::var("QUESTIONS_SERVICE_ADDR")
            .unwrap_or_else(|_| "questions-service:50052".to_string()),
        game_room_addr: env::var("GAME_ROOM_SERVICE_ADDR")
            .unwrap_or_else(|_| "game-room-service:50053".to_string()),
        score_addr: env::var("SCORE_SERVICE_ADDR")
            .unwrap_or_else(|_| "score-service:50054".to_string()),
    };
    let gateway_service = GatewayServerImpl {
        users_addr: state.users_addr.clone(),
        questions_addr: state.questions_addr.clone(),
        game_room_addr: state.game_room_addr.clone(),
        score_addr: state.score_addr.clone(),
    };

    let grpc_addr: SocketAddr = "0.0.0.0:50050".parse()?;
    let http_addr: SocketAddr = "0.0.0.0:8080".parse()?;

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin([
            "http://localhost:5173"
                .parse()
                .map_err(|e| anyhow!("CORS parse error: {e}"))?,
            "http://127.0.0.1:5173"
                .parse()
                .map_err(|e| anyhow!("CORS parse error: {e}"))?,
        ]);

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/users", post(create_user))
        .route("/api/login", post(login))
        .route("/api/questions/generate", post(generate_question))
        .route("/api/anime/search", get(search_anime))
        .route("/api/rooms", post(create_room))
        .route("/api/rooms/:room_id/join", post(join_room))
        .route("/api/rooms/:room_id/start", post(start_game))
        .route("/api/rooms/:room_id/answer", post(submit_answer))
        .route("/api/rooms/:room_id/leaderboard", get(get_leaderboard))
        .route("/api/rooms/:room_id/end", post(end_game))
        .layer(cors)
        .with_state(state);

    let grpc_server = async move {
        Server::builder()
            .add_service(GatewayServiceServer::new(gateway_service))
            .serve(grpc_addr)
            .await
            .map_err(|e| anyhow!("gRPC server error: {e}"))
    };

    let http_server = async move {
        let listener = TcpListener::bind(http_addr)
            .await
            .map_err(|e| anyhow!("HTTP bind error: {e}"))?;
        axum::serve(listener, app)
            .await
            .map_err(|e| anyhow!("HTTP server error: {e}"))
    };

    info!("api-gateway gRPC en {} y HTTP en {}", grpc_addr, http_addr);
    tokio::try_join!(grpc_server, http_server)?;
    Ok(())
}
