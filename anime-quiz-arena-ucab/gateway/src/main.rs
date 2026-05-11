use std::{env, net::SocketAddr};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tonic::{
    transport::{Channel, Server},
    Code, Request, Response, Status,
};
use tower_http::cors::CorsLayer;
use tracing::info;

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
    status: String,
    service: String,
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
#[serde(rename_all = "camelCase")]
struct AnimeDto {
    id: i32,
    title: String,
    synopsis: String,
    episodes: i32,
    score: f64,
    image_url: String,
}

#[derive(Debug, Serialize)]
struct SearchAnimeResponseDto {
    animes: Vec<AnimeDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Serialize)]
struct GenerateQuestionResponseDto {
    question: Option<QuestionDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RoomDto {
    id: String,
    name: String,
    status: String,
    created_by: String,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct RoomResponseDto {
    room: Option<RoomDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SubmitAnswerResponseDto {
    correct: bool,
    points_awarded: i32,
    total_points: i32,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScoreEntryDto {
    user_id: String,
    points: i32,
    rank: i32,
}

#[derive(Debug, Serialize)]
struct LeaderboardResponseDto {
    entries: Vec<ScoreEntryDto>,
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

#[derive(Debug, Deserialize)]
struct RoomIdBody {
    room_id: String,
}

#[derive(Debug, Deserialize)]
struct CreateRoomBody {
    name: String,
    created_by: String,
}

#[derive(Debug, Deserialize)]
struct JoinBody {
    user_id: String,
    username: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SubmitAnswerBody {
    user_id: String,
    question_id: String,
    selected_option: String,
    correct_option: String,
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: String,
}

#[derive(Debug, Deserialize)]
struct LeaderboardQuery {
    limit: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct RoomStateQuery {
    question_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RoomPlayerDto {
    user_id: String,
    username: String,
    answered_current_question: bool,
    ready: bool,
    joined_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RoomStateResponseDto {
    room: Option<RoomDto>,
    players: Vec<RoomPlayerDto>,
    total_players: i32,
    answered_count: i32,
    all_answered: bool,
}

impl From<users::User> for UserDto {
    fn from(user: users::User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            created_at: user.created_at,
        }
    }
}

impl From<questions::Anime> for AnimeDto {
    fn from(anime: questions::Anime) -> Self {
        Self {
            id: anime.id,
            title: anime.title,
            synopsis: anime.synopsis,
            episodes: anime.episodes,
            score: anime.score,
            image_url: anime.image_url,
        }
    }
}

impl From<questions::Question> for QuestionDto {
    fn from(question: questions::Question) -> Self {
        Self {
            id: question.id,
            text: question.text,
            option_a: question.option_a,
            option_b: question.option_b,
            option_c: question.option_c,
            option_d: question.option_d,
            correct_option: question.correct_option,
            anime_id: question.anime_id,
        }
    }
}

impl From<gameroom::Room> for RoomDto {
    fn from(room: gameroom::Room) -> Self {
        Self {
            id: room.id,
            name: room.name,
            status: room.status,
            created_by: room.created_by,
            created_at: room.created_at,
        }
    }
}

impl From<score::ScoreEntry> for ScoreEntryDto {
    fn from(entry: score::ScoreEntry) -> Self {
        Self {
            user_id: entry.user_id,
            points: entry.points,
            rank: entry.rank,
        }
    }
}

fn normalize_service_addr(addr: &str) -> String {
    if addr.starts_with("http://") || addr.starts_with("https://") {
        addr.to_string()
    } else {
        format!("http://{}", addr)
    }
}

fn map_grpc_error(status: Status) -> (StatusCode, Json<ApiErrorResponse>) {
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
        Code::Internal => "internal",
        _ => "unknown",
    };

    (
        http_status,
        Json(ApiErrorResponse {
            error: status.message().to_string(),
            code: code.to_string(),
        }),
    )
}

fn map_connect_error(err: tonic::transport::Error) -> (StatusCode, Json<ApiErrorResponse>) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ApiErrorResponse {
            error: format!("No se pudo conectar al servicio interno: {err}"),
            code: "unavailable".to_string(),
        }),
    )
}

impl GatewayServerImpl {
    async fn users_client(
        &self,
    ) -> Result<users::users_service_client::UsersServiceClient<Channel>, Status> {
        users::users_service_client::UsersServiceClient::connect(normalize_service_addr(
            &self.users_addr,
        ))
        .await
        .map_err(|_| Status::unavailable("users-service no disponible"))
    }

    async fn questions_client(
        &self,
    ) -> Result<questions::questions_service_client::QuestionsServiceClient<Channel>, Status> {
        questions::questions_service_client::QuestionsServiceClient::connect(normalize_service_addr(
            &self.questions_addr,
        ))
        .await
        .map_err(|_| Status::unavailable("questions-service no disponible"))
    }

    async fn game_room_client(
        &self,
    ) -> Result<gameroom::game_room_service_client::GameRoomServiceClient<Channel>, Status> {
        gameroom::game_room_service_client::GameRoomServiceClient::connect(normalize_service_addr(
            &self.game_room_addr,
        ))
        .await
        .map_err(|_| Status::unavailable("game-room-service no disponible"))
    }

    async fn score_client(
        &self,
    ) -> Result<score::score_service_client::ScoreServiceClient<Channel>, Status> {
        score::score_service_client::ScoreServiceClient::connect(normalize_service_addr(
            &self.score_addr,
        ))
        .await
        .map_err(|_| Status::unavailable("score-service no disponible"))
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
                username: p.username,
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

    async fn get_room_state(
        &self,
        request: Request<GatewayRoomStateRequest>,
    ) -> Result<Response<GatewayRoomStateResponse>, Status> {
        let p = request.into_inner();
        let mut c = self.game_room_client().await?;

        let r = c
            .get_room_state(gameroom::GetRoomStateRequest {
                room_id: p.room_id,
                current_question_id: p.current_question_id,
            })
            .await
            .map_err(|e| Status::new(e.code(), e.message().to_string()))?
            .into_inner();

        Ok(Response::new(GatewayRoomStateResponse {
            room: r.room.map(map_room),
            players: r
                .players
                .into_iter()
                .map(|p| GatewayRoomPlayer {
                    user_id: p.user_id,
                    username: p.username,
                    answered_current_question: p.answered_current_question,
                    ready: p.ready,
                    joined_at: p.joined_at,
                })
                .collect(),
            total_players: r.total_players,
            answered_count: r.answered_count,
            all_answered: r.all_answered,
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

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        service: "api-gateway".to_string(),
    })
}

async fn create_user_http(
    State(state): State<AppState>,
    Json(body): Json<CreateUserBody>,
) -> ApiResult<UserResponseDto> {
    let mut client = users::users_service_client::UsersServiceClient::connect(
        normalize_service_addr(&state.users_addr),
    )
    .await
    .map_err(map_connect_error)?;

    let response = client
        .create_user(users::CreateUserRequest {
            username: body.username,
            email: body.email,
            password: body.password,
        })
        .await
        .map_err(map_grpc_error)?
        .into_inner();

    Ok(Json(UserResponseDto {
        user: response.user.map(UserDto::from),
    }))
}

async fn login_http(
    State(state): State<AppState>,
    Json(body): Json<LoginBody>,
) -> ApiResult<LoginResponseDto> {
    let mut client = users::users_service_client::UsersServiceClient::connect(
        normalize_service_addr(&state.users_addr),
    )
    .await
    .map_err(map_connect_error)?;

    let response = client
        .login_basic(users::LoginBasicRequest {
            email: body.email,
            password: body.password,
        })
        .await
        .map_err(map_grpc_error)?
        .into_inner();

    Ok(Json(LoginResponseDto {
        success: response.success,
        user: response.user.map(UserDto::from),
    }))
}

async fn generate_question_http(
    State(state): State<AppState>,
    Json(body): Json<RoomIdBody>,
) -> ApiResult<GenerateQuestionResponseDto> {
    let mut client = questions::questions_service_client::QuestionsServiceClient::connect(
        normalize_service_addr(&state.questions_addr),
    )
    .await
    .map_err(map_connect_error)?;

    let response = client
        .generate_question(questions::GenerateQuestionRequest {
            room_id: body.room_id,
        })
        .await
        .map_err(map_grpc_error)?
        .into_inner();

    Ok(Json(GenerateQuestionResponseDto {
        question: response.question.map(QuestionDto::from),
    }))
}

async fn search_anime_http(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> ApiResult<SearchAnimeResponseDto> {
    let mut client = questions::questions_service_client::QuestionsServiceClient::connect(
        normalize_service_addr(&state.questions_addr),
    )
    .await
    .map_err(map_connect_error)?;

    let response = client
        .search_anime(questions::SearchAnimeRequest { query: query.q })
        .await
        .map_err(map_grpc_error)?
        .into_inner();

    Ok(Json(SearchAnimeResponseDto {
        animes: response.animes.into_iter().map(AnimeDto::from).collect(),
    }))
}

async fn create_room_http(
    State(state): State<AppState>,
    Json(body): Json<CreateRoomBody>,
) -> ApiResult<RoomResponseDto> {
    let mut client = gameroom::game_room_service_client::GameRoomServiceClient::connect(
        normalize_service_addr(&state.game_room_addr),
    )
    .await
    .map_err(map_connect_error)?;

    let response = client
        .create_room(gameroom::CreateRoomRequest {
            name: body.name,
            created_by: body.created_by,
        })
        .await
        .map_err(map_grpc_error)?
        .into_inner();

    Ok(Json(RoomResponseDto {
        room: response.room.map(RoomDto::from),
    }))
}

async fn join_room_http(
    Path(room_id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<JoinBody>,
) -> ApiResult<RoomResponseDto> {
    let mut client = gameroom::game_room_service_client::GameRoomServiceClient::connect(
        normalize_service_addr(&state.game_room_addr),
    )
    .await
    .map_err(map_connect_error)?;

    let response = client
        .join_room(gameroom::JoinRoomRequest {
            room_id,
            user_id: body.user_id,
            username: body.username.unwrap_or_default(),
        })
        .await
        .map_err(map_grpc_error)?
        .into_inner();

    Ok(Json(RoomResponseDto {
        room: response.room.map(RoomDto::from),
    }))
}

async fn start_game_http(
    Path(room_id): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<RoomResponseDto> {
    let mut client = gameroom::game_room_service_client::GameRoomServiceClient::connect(
        normalize_service_addr(&state.game_room_addr),
    )
    .await
    .map_err(map_connect_error)?;

    let response = client
        .start_game(gameroom::StartGameRequest { room_id })
        .await
        .map_err(map_grpc_error)?
        .into_inner();

    Ok(Json(RoomResponseDto {
        room: response.room.map(RoomDto::from),
    }))
}

async fn submit_answer_http(
    Path(room_id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<SubmitAnswerBody>,
) -> ApiResult<SubmitAnswerResponseDto> {
    let mut client = gameroom::game_room_service_client::GameRoomServiceClient::connect(
        normalize_service_addr(&state.game_room_addr),
    )
    .await
    .map_err(map_connect_error)?;

    let response = client
        .submit_answer(gameroom::SubmitAnswerRequest {
            room_id,
            user_id: body.user_id,
            question_id: body.question_id,
            selected_option: body.selected_option,
            correct_option: body.correct_option,
        })
        .await
        .map_err(map_grpc_error)?
        .into_inner();

    Ok(Json(SubmitAnswerResponseDto {
        correct: response.correct,
        points_awarded: response.points_awarded,
        total_points: response.total_points,
        message: response.message,
    }))
}

async fn leaderboard_http(
    Path(room_id): Path<String>,
    State(state): State<AppState>,
    Query(query): Query<LeaderboardQuery>,
) -> ApiResult<LeaderboardResponseDto> {
    let mut client = score::score_service_client::ScoreServiceClient::connect(
        normalize_service_addr(&state.score_addr),
    )
    .await
    .map_err(map_connect_error)?;

    let response = client
        .get_leaderboard(score::GetLeaderboardRequest {
            room_id,
            limit: query.limit.unwrap_or(10),
        })
        .await
        .map_err(map_grpc_error)?
        .into_inner();

    Ok(Json(LeaderboardResponseDto {
        entries: response
            .entries
            .into_iter()
            .map(ScoreEntryDto::from)
            .collect(),
    }))
}

async fn end_game_http(
    Path(room_id): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<RoomResponseDto> {
    let mut client = gameroom::game_room_service_client::GameRoomServiceClient::connect(
        normalize_service_addr(&state.game_room_addr),
    )
    .await
    .map_err(map_connect_error)?;

    let response = client
        .end_game(gameroom::EndGameRequest { room_id })
        .await
        .map_err(map_grpc_error)?
        .into_inner();

    Ok(Json(RoomResponseDto {
        room: response.room.map(RoomDto::from),
    }))
}

async fn room_state_http(
    Path(room_id): Path<String>,
    State(state): State<AppState>,
    Query(query): Query<RoomStateQuery>,
) -> ApiResult<RoomStateResponseDto> {
    let mut client = gameroom::game_room_service_client::GameRoomServiceClient::connect(
        normalize_service_addr(&state.game_room_addr),
    )
    .await
    .map_err(map_connect_error)?;

    let response = client
        .get_room_state(gameroom::GetRoomStateRequest {
            room_id,
            current_question_id: query.question_id.unwrap_or_default(),
        })
        .await
        .map_err(map_grpc_error)?
        .into_inner();

    Ok(Json(RoomStateResponseDto {
        room: response.room.map(RoomDto::from),
        players: response
            .players
            .into_iter()
            .map(|p| RoomPlayerDto {
                user_id: p.user_id,
                username: p.username,
                answered_current_question: p.answered_current_question,
                ready: p.ready,
                joined_at: p.joined_at,
            })
            .collect(),
        total_players: response.total_players,
        answered_count: response.answered_count,
        all_answered: response.all_answered,
    }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let users_addr =
        env::var("USERS_SERVICE_ADDR").unwrap_or_else(|_| "users-service:50051".to_string());

    let questions_addr = env::var("QUESTIONS_SERVICE_ADDR")
        .unwrap_or_else(|_| "questions-service:50052".to_string());

    let game_room_addr = env::var("GAME_ROOM_SERVICE_ADDR")
        .unwrap_or_else(|_| "game-room-service:50053".to_string());

    let score_addr =
        env::var("SCORE_SERVICE_ADDR").unwrap_or_else(|_| "score-service:50054".to_string());

    info!("users-service addr: {}", normalize_service_addr(&users_addr));
    info!(
        "questions-service addr: {}",
        normalize_service_addr(&questions_addr)
    );
    info!(
        "game-room-service addr: {}",
        normalize_service_addr(&game_room_addr)
    );
    info!("score-service addr: {}", normalize_service_addr(&score_addr));

    let grpc_gateway = GatewayServerImpl {
        users_addr: users_addr.clone(),
        questions_addr: questions_addr.clone(),
        game_room_addr: game_room_addr.clone(),
        score_addr: score_addr.clone(),
    };

    let app_state = AppState {
        users_addr,
        questions_addr,
        game_room_addr,
        score_addr,
    };

    let grpc_port = env::var("GRPC_PORT").unwrap_or_else(|_| "50050".to_string());
    let http_port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());

    let grpc_addr: SocketAddr = format!("0.0.0.0:{grpc_port}").parse()?;
    let http_addr: SocketAddr = format!("0.0.0.0:{http_port}").parse()?;

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/users", post(create_user_http))
        .route("/api/login", post(login_http))
        .route("/api/questions/generate", post(generate_question_http))
        .route("/api/anime/search", get(search_anime_http))
        .route("/api/rooms", post(create_room_http))
        .route("/api/rooms/:room_id/join", post(join_room_http))
        .route("/api/rooms/:room_id/start", post(start_game_http))
        .route("/api/rooms/:room_id/answer", post(submit_answer_http))
        .route("/api/rooms/:room_id/leaderboard", get(leaderboard_http))
        .route("/api/rooms/:room_id/state", get(room_state_http))
        .route("/api/rooms/:room_id/end", post(end_game_http))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    info!("api-gateway gRPC en {} y HTTP en {}", grpc_addr, http_addr);

    let grpc_task = async move {
        Server::builder()
            .add_service(GatewayServiceServer::new(grpc_gateway))
            .serve(grpc_addr)
            .await
            .map_err(|e| anyhow::anyhow!("gRPC server error: {e}"))
    };

    let http_task = async move {
        let listener = TcpListener::bind(http_addr)
            .await
            .map_err(|e| anyhow::anyhow!("HTTP bind error: {e}"))?;

        axum::serve(listener, app)
            .await
            .map_err(|e| anyhow::anyhow!("HTTP server error: {e}"))
    };

    tokio::try_join!(grpc_task, http_task)?;

    Ok(())
}