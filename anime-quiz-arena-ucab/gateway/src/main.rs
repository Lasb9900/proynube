use std::{env, net::SocketAddr};
use axum::{extract::{Path, Query, State}, http::{Method, StatusCode}, response::IntoResponse, routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tonic::{transport::Channel, transport::Server, Code, Request, Response, Status};
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};

pub mod gateway { tonic::include_proto!("animequiz.gateway.v1"); }
pub mod users { tonic::include_proto!("animequiz.users.v1"); }
pub mod questions { tonic::include_proto!("animequiz.questions.v1"); }
pub mod gameroom { tonic::include_proto!("animequiz.gameroom.v1"); }
pub mod score { tonic::include_proto!("animequiz.score.v1"); }

use gateway::gateway_service_server::{GatewayService, GatewayServiceServer};
use gateway::*;

#[derive(Clone)]
struct GatewayServerImpl { users_addr: String, questions_addr: String, game_room_addr: String, score_addr: String }

#[derive(Serialize)] struct HealthResponse { status: &'static str, service: &'static str }
#[derive(Serialize)] struct ApiErrorResponse { error: String, code: String }
impl IntoResponse for ApiErrorResponse { fn into_response(self) -> axum::response::Response { let status = match self.code.as_str(){"invalid_argument"=>StatusCode::BAD_REQUEST,"unauthenticated"=>StatusCode::UNAUTHORIZED,"not_found"=>StatusCode::NOT_FOUND,"already_exists"=>StatusCode::CONFLICT,"failed_precondition"=>StatusCode::PRECONDITION_FAILED,"unavailable"=>StatusCode::SERVICE_UNAVAILABLE,_=>StatusCode::INTERNAL_SERVER_ERROR}; (status, Json(self)).into_response() } }
fn map_grpc_error(s: Status) -> ApiErrorResponse { ApiErrorResponse{ error: s.message().to_string(), code: match s.code(){Code::InvalidArgument=>"invalid_argument",Code::Unauthenticated=>"unauthenticated",Code::NotFound=>"not_found",Code::AlreadyExists=>"already_exists",Code::FailedPrecondition=>"failed_precondition",Code::Unavailable=>"unavailable",Code::Internal=>"internal",_=>"unknown"}.to_string() } }

impl GatewayServerImpl {
    async fn users_client(&self) -> Result<users::users_service_client::UsersServiceClient<Channel>, Status> { users::users_service_client::UsersServiceClient::connect(format!("http://{}", self.users_addr)).await.map_err(|_| Status::unavailable("users-service no disponible")) }
    async fn questions_client(&self) -> Result<questions::questions_service_client::QuestionsServiceClient<Channel>, Status> { questions::questions_service_client::QuestionsServiceClient::connect(format!("http://{}", self.questions_addr)).await.map_err(|_| Status::unavailable("questions-service no disponible")) }
    async fn game_room_client(&self) -> Result<gameroom::game_room_service_client::GameRoomServiceClient<Channel>, Status> { gameroom::game_room_service_client::GameRoomServiceClient::connect(format!("http://{}", self.game_room_addr)).await.map_err(|_| Status::unavailable("game-room-service no disponible")) }
    async fn score_client(&self) -> Result<score::score_service_client::ScoreServiceClient<Channel>, Status> { score::score_service_client::ScoreServiceClient::connect(format!("http://{}", self.score_addr)).await.map_err(|_| Status::unavailable("score-service no disponible")) }
}

#[tonic::async_trait]
impl GatewayService for GatewayServerImpl {
    async fn create_user(&self, request: Request<GatewayCreateUserRequest>) -> Result<Response<GatewayUserResponse>, Status> { let p=request.into_inner(); let mut c=self.users_client().await?; let r=c.create_user(users::CreateUserRequest{username:p.username,email:p.email,password:p.password}).await.map_err(|e| Status::new(e.code(), e.message().to_string()))?.into_inner(); Ok(Response::new(GatewayUserResponse{ user:r.user.map(|u| GatewayUser{id:u.id,username:u.username,email:u.email,created_at:u.created_at}) })) }
    async fn login_basic(&self, request: Request<GatewayLoginRequest>) -> Result<Response<GatewayLoginResponse>, Status> { let p=request.into_inner(); let mut c=self.users_client().await?; let r=c.login_basic(users::LoginBasicRequest{email:p.email,password:p.password}).await.map_err(|e| Status::new(e.code(), e.message().to_string()))?.into_inner(); Ok(Response::new(GatewayLoginResponse{success:r.success,user:r.user.map(|u| GatewayUser{id:u.id,username:u.username,email:u.email,created_at:u.created_at})})) }
    async fn search_anime(&self, request: Request<GatewaySearchAnimeRequest>) -> Result<Response<GatewaySearchAnimeResponse>, Status> { let p=request.into_inner(); let mut c=self.questions_client().await?; let r=c.search_anime(questions::SearchAnimeRequest{query:p.query}).await.map_err(|e| Status::new(e.code(), e.message().to_string()))?.into_inner(); Ok(Response::new(GatewaySearchAnimeResponse{animes:r.animes.into_iter().map(|a| GatewayAnime{id:a.id,title:a.title,synopsis:a.synopsis,episodes:a.episodes,score:a.score,image_url:a.image_url}).collect()})) }
    async fn generate_question(&self, request: Request<GatewayGenerateQuestionRequest>) -> Result<Response<GatewayGenerateQuestionResponse>, Status> { let p=request.into_inner(); let mut c=self.questions_client().await?; let r=c.generate_question(questions::GenerateQuestionRequest{room_id:p.room_id}).await.map_err(|e| Status::new(e.code(), e.message().to_string()))?.into_inner(); Ok(Response::new(GatewayGenerateQuestionResponse{question:r.question.map(|q| GatewayQuestion{id:q.id,text:q.text,option_a:q.option_a,option_b:q.option_b,option_c:q.option_c,option_d:q.option_d,correct_option:q.correct_option,anime_id:q.anime_id})})) }
    async fn create_room(&self, request: Request<GatewayCreateRoomRequest>) -> Result<Response<GatewayRoomResponse>, Status> { let p=request.into_inner(); let mut c=self.game_room_client().await?; let r=c.create_room(gameroom::CreateRoomRequest{name:p.name,created_by:p.created_by}).await.map_err(|e| Status::new(e.code(), e.message().to_string()))?.into_inner(); Ok(Response::new(GatewayRoomResponse{room:r.room.map(map_room)})) }
    async fn join_room(&self, request: Request<GatewayJoinRoomRequest>) -> Result<Response<GatewayRoomResponse>, Status> { let p=request.into_inner(); let mut c=self.game_room_client().await?; let r=c.join_room(gameroom::JoinRoomRequest{room_id:p.room_id,user_id:p.user_id}).await.map_err(|e| Status::new(e.code(), e.message().to_string()))?.into_inner(); Ok(Response::new(GatewayRoomResponse{room:r.room.map(map_room)})) }
    async fn start_game(&self, request: Request<GatewayRoomIdRequest>) -> Result<Response<GatewayRoomResponse>, Status> { let p=request.into_inner(); let mut c=self.game_room_client().await?; let r=c.start_game(gameroom::StartGameRequest{room_id:p.room_id}).await.map_err(|e| Status::new(e.code(), e.message().to_string()))?.into_inner(); Ok(Response::new(GatewayRoomResponse{room:r.room.map(map_room)})) }
    async fn end_game(&self, request: Request<GatewayRoomIdRequest>) -> Result<Response<GatewayRoomResponse>, Status> { let p=request.into_inner(); let mut c=self.game_room_client().await?; let r=c.end_game(gameroom::EndGameRequest{room_id:p.room_id}).await.map_err(|e| Status::new(e.code(), e.message().to_string()))?.into_inner(); Ok(Response::new(GatewayRoomResponse{room:r.room.map(map_room)})) }
    async fn submit_answer(&self, request: Request<GatewaySubmitAnswerRequest>) -> Result<Response<GatewaySubmitAnswerResponse>, Status> { let p=request.into_inner(); let mut c=self.game_room_client().await?; let r=c.submit_answer(gameroom::SubmitAnswerRequest{room_id:p.room_id,user_id:p.user_id,question_id:p.question_id,selected_option:p.selected_option,correct_option:p.correct_option}).await.map_err(|e| Status::new(e.code(), e.message().to_string()))?.into_inner(); Ok(Response::new(GatewaySubmitAnswerResponse{correct:r.correct,points_awarded:r.points_awarded,total_points:r.total_points,message:r.message})) }
    async fn get_leaderboard(&self, request: Request<GatewayLeaderboardRequest>) -> Result<Response<GatewayLeaderboardResponse>, Status> { let p=request.into_inner(); let mut c=self.score_client().await?; let r=c.get_leaderboard(score::GetLeaderboardRequest{room_id:p.room_id,limit:p.limit}).await.map_err(|e| Status::new(e.code(), e.message().to_string()))?.into_inner(); Ok(Response::new(GatewayLeaderboardResponse{entries:r.entries.into_iter().map(|e| GatewayScoreEntry{user_id:e.user_id,points:e.points,rank:e.rank}).collect()})) }
}
fn map_room(room: gameroom::Room) -> GatewayRoom { GatewayRoom{id:room.id,name:room.name,status:room.status,created_by:room.created_by,created_at:room.created_at} }

#[derive(Deserialize)] struct CreateUserBody { username: String, email: String, password: String }
#[derive(Deserialize)] struct LoginBody { email: String, password: String }
#[derive(Deserialize)] struct RoomIdBody { room_id: String }
#[derive(Deserialize)] struct CreateRoomBody { name: String, created_by: String }
#[derive(Deserialize)] struct JoinBody { user_id: String }
#[derive(Deserialize)] struct SubmitAnswerBody { user_id: String, question_id: String, selected_option: String, correct_option: String }
#[derive(Deserialize)] struct SearchQuery { q: String }
#[derive(Deserialize)] struct LeaderboardQuery { limit: Option<i32> }

async fn health() -> Json<HealthResponse> { Json(HealthResponse{status:"ok",service:"api-gateway"}) }
async fn create_user_http(State(s):State<GatewayServerImpl>, Json(b):Json<CreateUserBody>) -> Result<Json<GatewayUserResponse>, ApiErrorResponse> { Ok(Json(GatewayService::create_user(&s, Request::new(GatewayCreateUserRequest{username:b.username,email:b.email,password:b.password})).await.map_err(map_grpc_error)?.into_inner())) }
async fn login_http(State(s):State<GatewayServerImpl>, Json(b):Json<LoginBody>) -> Result<Json<GatewayLoginResponse>, ApiErrorResponse> { Ok(Json(GatewayService::login_basic(&s, Request::new(GatewayLoginRequest{email:b.email,password:b.password})).await.map_err(map_grpc_error)?.into_inner())) }
async fn generate_question_http(State(s):State<GatewayServerImpl>, Json(b):Json<RoomIdBody>) -> Result<Json<GatewayGenerateQuestionResponse>, ApiErrorResponse> { Ok(Json(GatewayService::generate_question(&s, Request::new(GatewayGenerateQuestionRequest{room_id:b.room_id})).await.map_err(map_grpc_error)?.into_inner())) }
async fn search_anime_http(State(s):State<GatewayServerImpl>, Query(q):Query<SearchQuery>) -> Result<Json<GatewaySearchAnimeResponse>, ApiErrorResponse> { Ok(Json(GatewayService::search_anime(&s, Request::new(GatewaySearchAnimeRequest{query:q.q})).await.map_err(map_grpc_error)?.into_inner())) }
async fn create_room_http(State(s):State<GatewayServerImpl>, Json(b):Json<CreateRoomBody>) -> Result<Json<GatewayRoomResponse>, ApiErrorResponse> { Ok(Json(GatewayService::create_room(&s, Request::new(GatewayCreateRoomRequest{name:b.name,created_by:b.created_by})).await.map_err(map_grpc_error)?.into_inner())) }
async fn join_room_http(Path(room_id):Path<String>, State(s):State<GatewayServerImpl>, Json(b):Json<JoinBody>) -> Result<Json<GatewayRoomResponse>, ApiErrorResponse> { Ok(Json(GatewayService::join_room(&s, Request::new(GatewayJoinRoomRequest{room_id,user_id:b.user_id})).await.map_err(map_grpc_error)?.into_inner())) }
async fn start_game_http(Path(room_id):Path<String>, State(s):State<GatewayServerImpl>) -> Result<Json<GatewayRoomResponse>, ApiErrorResponse> { Ok(Json(GatewayService::start_game(&s, Request::new(GatewayRoomIdRequest{room_id})).await.map_err(map_grpc_error)?.into_inner())) }
async fn submit_answer_http(Path(room_id):Path<String>, State(s):State<GatewayServerImpl>, Json(b):Json<SubmitAnswerBody>) -> Result<Json<GatewaySubmitAnswerResponse>, ApiErrorResponse> { Ok(Json(GatewayService::submit_answer(&s, Request::new(GatewaySubmitAnswerRequest{room_id,user_id:b.user_id,question_id:b.question_id,selected_option:b.selected_option,correct_option:b.correct_option})).await.map_err(map_grpc_error)?.into_inner())) }
async fn leaderboard_http(Path(room_id):Path<String>, State(s):State<GatewayServerImpl>, Query(q):Query<LeaderboardQuery>) -> Result<Json<GatewayLeaderboardResponse>, ApiErrorResponse> { Ok(Json(GatewayService::get_leaderboard(&s, Request::new(GatewayLeaderboardRequest{room_id,limit:q.limit.unwrap_or(10)})).await.map_err(map_grpc_error)?.into_inner())) }
async fn end_game_http(Path(room_id):Path<String>, State(s):State<GatewayServerImpl>) -> Result<Json<GatewayRoomResponse>, ApiErrorResponse> { Ok(Json(GatewayService::end_game(&s, Request::new(GatewayRoomIdRequest{room_id})).await.map_err(map_grpc_error)?.into_inner())) }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();
    let gateway = GatewayServerImpl { users_addr: env::var("USERS_SERVICE_ADDR").unwrap_or_else(|_| "users-service:50051".to_string()), questions_addr: env::var("QUESTIONS_SERVICE_ADDR").unwrap_or_else(|_| "questions-service:50052".to_string()), game_room_addr: env::var("GAME_ROOM_SERVICE_ADDR").unwrap_or_else(|_| "game-room-service:50053".to_string()), score_addr: env::var("SCORE_SERVICE_ADDR").unwrap_or_else(|_| "score-service:50054".to_string()) };

    let grpc_addr: SocketAddr = "0.0.0.0:50050".parse()?;
    let http_addr: SocketAddr = "0.0.0.0:8080".parse()?;

    let cors = CorsLayer::new().allow_methods([Method::GET, Method::POST]).allow_headers(Any).allow_origin([
        "http://localhost:5173".parse()?,
        "http://127.0.0.1:5173".parse()?,
    ]);

    let app = Router::new().route("/health", get(health)).route("/api/users", post(create_user_http)).route("/api/login", post(login_http)).route("/api/questions/generate", post(generate_question_http)).route("/api/anime/search", get(search_anime_http)).route("/api/rooms", post(create_room_http)).route("/api/rooms/:room_id/join", post(join_room_http)).route("/api/rooms/:room_id/start", post(start_game_http)).route("/api/rooms/:room_id/answer", post(submit_answer_http)).route("/api/rooms/:room_id/leaderboard", get(leaderboard_http)).route("/api/rooms/:room_id/end", post(end_game_http)).layer(cors).with_state(gateway.clone());

    let grpc_task = async move { Server::builder().add_service(GatewayServiceServer::new(gateway)).serve(grpc_addr).await };
    let http_task = async move { let listener = TcpListener::bind(http_addr).await?; axum::serve(listener, app).await };

    info!("api-gateway gRPC en {} y HTTP en {}", grpc_addr, http_addr);
    tokio::try_join!(grpc_task, http_task)?;
    Ok(())
}
