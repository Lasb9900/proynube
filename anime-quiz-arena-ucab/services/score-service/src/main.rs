use tonic::{transport::Server, Request, Response, Status};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    tracing::info!("skeleton service booted");
    Ok(())
}
