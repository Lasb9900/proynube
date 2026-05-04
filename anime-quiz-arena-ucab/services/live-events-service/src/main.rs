use std::pin::Pin;

use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status};
use tracing::{info, warn};

pub mod live_events {
    tonic::include_proto!("animequiz.liveevents.v1");
}

use live_events::live_events_service_server::{LiveEventsService, LiveEventsServiceServer};
use live_events::{GameEvent, GameStreamRequest, GameStreamResponse};

type ResponseStream =
    Pin<Box<dyn tokio_stream::Stream<Item = Result<GameStreamResponse, Status>> + Send>>;

#[derive(Default)]
struct LiveEventsSvc;

fn validate_event(event: &GameEvent) -> Result<(), Status> {
    if event.room_id.trim().is_empty() {
        return Err(Status::invalid_argument("room_id must not be empty"));
    }
    if event.event_type.trim().is_empty() {
        return Err(Status::invalid_argument("event_type must not be empty"));
    }
    Ok(())
}

fn event_ack_message(event_type: &str) -> &'static str {
    match event_type {
        "PlayerJoined" => "Player joined room",
        "QuestionStarted" => "Question started",
        "AnswerSubmitted" => "Answer received",
        "ScoreUpdated" => "Score updated",
        "GameFinished" => "Game finished",
        _ => "Unknown event type",
    }
}

#[tonic::async_trait]
impl LiveEventsService for LiveEventsSvc {
    type GameStreamStream = ResponseStream;

    async fn game_stream(
        &self,
        request: Request<tonic::Streaming<GameStreamRequest>>,
    ) -> Result<Response<Self::GameStreamStream>, Status> {
        let mut inbound = request.into_inner();
        let (tx, rx) = mpsc::channel(16);

        tokio::spawn(async move {
            while let Some(next) = inbound.message().await.transpose() {
                let response = match next {
                    Ok(req) => {
                        let event = match req.event {
                            Some(event) => event,
                            None => {
                                if tx
                                    .send(Err(Status::invalid_argument("event must be provided")))
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                                continue;
                            }
                        };

                        if let Err(validation_error) = validate_event(&event) {
                            if tx.send(Err(validation_error)).await.is_err() {
                                break;
                            }
                            continue;
                        }

                        info!(
                            room_id = %event.room_id,
                            user_id = %event.user_id,
                            event_type = %event.event_type,
                            points = event.points,
                            message = %event.message,
                            timestamp = %event.timestamp,
                            "Game event received"
                        );

                        let ack = event_ack_message(event.event_type.as_str());
                        if ack == "Unknown event type" {
                            warn!(event_type = %event.event_type, "Unknown event type");
                        }

                        Ok(GameStreamResponse {
                            event: Some(GameEvent {
                                room_id: event.room_id,
                                user_id: event.user_id,
                                event_type: event.event_type,
                                message: ack.to_string(),
                                points: event.points,
                                timestamp: event.timestamp,
                            }),
                        })
                    }
                    Err(e) => Err(Status::internal(format!("stream read error: {e}"))),
                };

                if tx.send(response).await.is_err() {
                    break;
                }
            }

            info!("Client stream closed for GameStream");
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let addr = "0.0.0.0:50055".parse()?;
    info!("Starting live-events-service on 0.0.0.0:50055");

    Server::builder()
        .add_service(LiveEventsServiceServer::new(LiveEventsSvc))
        .serve(addr)
        .await?;

    Ok(())
}
