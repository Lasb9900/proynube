use redis::AsyncCommands;
use std::{env, pin::Pin, time::Duration};
use tokio::{sync::mpsc, time::interval};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status};
use tracing::{error, info};

pub mod score {
    tonic::include_proto!("animequiz.score.v1");
}

use score::score_service_server::{ScoreService, ScoreServiceServer};
use score::{
    AddScoreRequest, AddScoreResponse, GetLeaderboardRequest, GetLeaderboardResponse,
    GetScoreRequest, GetScoreResponse, ScoreEntry, WatchLeaderboardRequest,
};

#[derive(Clone)]
struct ScoreSvc {
    redis_client: redis::Client,
}

type LeaderboardStream =
    Pin<Box<dyn tokio_stream::Stream<Item = Result<GetLeaderboardResponse, Status>> + Send>>;

impl ScoreSvc {
    fn leaderboard_key(room_id: &str) -> String {
        format!("leaderboard:{room_id}")
    }

    fn validate_room_user(room_id: &str, user_id: &str) -> Result<(), Status> {
        if room_id.trim().is_empty() {
            return Err(Status::invalid_argument("room_id must not be empty"));
        }
        if user_id.trim().is_empty() {
            return Err(Status::invalid_argument("user_id must not be empty"));
        }
        Ok(())
    }

    fn normalize_limit(limit: i32) -> isize {
        if limit <= 0 {
            10
        } else {
            limit as isize
        }
    }

    async fn fetch_leaderboard(
        &self,
        room_id: &str,
        limit: i32,
    ) -> Result<Vec<ScoreEntry>, Status> {
        let key = Self::leaderboard_key(room_id);
        let normalized_limit = Self::normalize_limit(limit);
        let mut conn = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| Status::unavailable(format!("redis connection error: {e}")))?;

        let raw: Vec<(String, f64)> = conn
            .zrevrange_withscores(&key, 0, normalized_limit - 1)
            .await
            .map_err(|e| Status::unavailable(format!("redis query error: {e}")))?;

        info!(room_id = %room_id, limit = normalized_limit, "Leaderboard queried");

        Ok(raw
            .into_iter()
            .enumerate()
            .map(|(idx, (user_id, points))| ScoreEntry {
                user_id,
                points: points as i32,
                rank: (idx + 1) as i32,
            })
            .collect())
    }
}

#[tonic::async_trait]
impl ScoreService for ScoreSvc {
    async fn add_score(
        &self,
        request: Request<AddScoreRequest>,
    ) -> Result<Response<AddScoreResponse>, Status> {
        let payload = request.into_inner();
        Self::validate_room_user(&payload.room_id, &payload.user_id)?;

        let key = Self::leaderboard_key(&payload.room_id);
        let mut conn = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| Status::unavailable(format!("redis connection error: {e}")))?;

        let total_points: f64 = conn
            .zincr(&key, &payload.user_id, payload.points as f64)
            .await
            .map_err(|e| Status::unavailable(format!("redis write error: {e}")))?;

        info!(room_id = %payload.room_id, user_id = %payload.user_id, points = payload.points, total_points = total_points, "Score added");

        Ok(Response::new(AddScoreResponse {
            user_id: payload.user_id,
            total_points: total_points as i32,
        }))
    }

    async fn get_score(
        &self,
        request: Request<GetScoreRequest>,
    ) -> Result<Response<GetScoreResponse>, Status> {
        let payload = request.into_inner();
        Self::validate_room_user(&payload.room_id, &payload.user_id)?;

        let key = Self::leaderboard_key(&payload.room_id);
        let mut conn = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| Status::unavailable(format!("redis connection error: {e}")))?;

        let points: Option<f64> = redis::cmd("ZSCORE")
            .arg(&key)
            .arg(&payload.user_id)
            .query_async(&mut conn)
            .await
            .map_err(|e| Status::unavailable(format!("redis query error: {e}")))?;

        Ok(Response::new(GetScoreResponse {
            user_id: payload.user_id,
            points: points.unwrap_or(0.0) as i32,
        }))
    }

    async fn get_leaderboard(
        &self,
        request: Request<GetLeaderboardRequest>,
    ) -> Result<Response<GetLeaderboardResponse>, Status> {
        let payload = request.into_inner();
        if payload.room_id.trim().is_empty() {
            return Err(Status::invalid_argument("room_id must not be empty"));
        }

        let entries = self
            .fetch_leaderboard(&payload.room_id, payload.limit)
            .await?;
        Ok(Response::new(GetLeaderboardResponse { entries }))
    }

    type WatchLeaderboardStream = LeaderboardStream;

    async fn watch_leaderboard(
        &self,
        request: Request<WatchLeaderboardRequest>,
    ) -> Result<Response<Self::WatchLeaderboardStream>, Status> {
        let payload = request.into_inner();
        if payload.room_id.trim().is_empty() {
            return Err(Status::invalid_argument("room_id must not be empty"));
        }

        let (tx, rx) = mpsc::channel(8);
        let svc = self.clone();
        let room_id = payload.room_id;
        let limit = payload.limit;

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(2));
            for _ in 0..10 {
                ticker.tick().await;
                let send_result = match svc.fetch_leaderboard(&room_id, limit).await {
                    Ok(entries) => tx.send(Ok(GetLeaderboardResponse { entries })).await,
                    Err(e) => tx.send(Err(e)).await,
                };

                if send_result.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let addr = "0.0.0.0:50054".parse()?;
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://redis:6379".to_string());

    info!("Starting score-service on 0.0.0.0:50054");
    let redis_client = redis::Client::open(redis_url.as_str())?;

    let mut conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to connect to Redis");
            e
        })?;
    let pong: String = redis::cmd("PING").query_async(&mut conn).await?;
    info!(pong = %pong, "Connected to Redis");

    let service = ScoreSvc { redis_client };

    Server::builder()
        .add_service(ScoreServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
