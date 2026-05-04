use rand::seq::SliceRandom;
use rand::Rng;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tonic::{transport::Server, Request, Response, Status};
use tracing::{error, info, warn};
use uuid::Uuid;

pub mod questions {
    tonic::include_proto!("animequiz.questions.v1");
}

use questions::questions_service_server::{QuestionsService, QuestionsServiceServer};
use questions::{
    Anime, GenerateQuestionRequest, GenerateQuestionResponse, GetAnimeByIdRequest,
    GetAnimeByIdResponse, Question, SearchAnimeRequest, SearchAnimeResponse,
};

#[derive(Clone)]
struct QuestionsSvc {
    adapter: Arc<JikanAdapter>,
}

#[derive(Debug)]
struct CircuitState {
    consecutive_failures: u8,
    open_until: Option<Instant>,
}

#[derive(Clone)]
struct JikanAdapter {
    client: Client,
    base_url: String,
    circuit: Arc<Mutex<CircuitState>>,
    cooldown: Duration,
}

#[derive(Deserialize)]
struct JikanListResponse {
    data: Vec<JikanAnime>,
}

#[derive(Deserialize)]
struct JikanSingleResponse {
    data: JikanAnime,
}

#[derive(Deserialize)]
struct JikanAnime {
    mal_id: i32,
    title: String,
    synopsis: Option<String>,
    episodes: Option<i32>,
    score: Option<f64>,
    images: Option<JikanImages>,
}

#[derive(Deserialize)]
struct JikanImages {
    jpg: Option<JikanJpg>,
}

#[derive(Deserialize)]
struct JikanJpg {
    image_url: Option<String>,
}

impl JikanAdapter {
    fn new(
        base_url: String,
        timeout: Duration,
        cooldown: Duration,
    ) -> Result<Self, reqwest::Error> {
        let client = Client::builder().timeout(timeout).build()?;
        Ok(Self {
            client,
            base_url,
            circuit: Arc::new(Mutex::new(CircuitState {
                consecutive_failures: 0,
                open_until: None,
            })),
            cooldown,
        })
    }

    async fn should_short_circuit(&self) -> bool {
        let mut state = self.circuit.lock().await;
        if let Some(until) = state.open_until {
            if Instant::now() < until {
                return true;
            }
            info!("Circuit breaker cooldown ended; switching to half-open trial.");
            state.open_until = None;
            state.consecutive_failures = 0;
        }
        false
    }

    async fn record_success(&self) {
        let mut state = self.circuit.lock().await;
        state.consecutive_failures = 0;
        state.open_until = None;
    }

    async fn record_failure(&self) {
        let mut state = self.circuit.lock().await;
        state.consecutive_failures = state.consecutive_failures.saturating_add(1);
        warn!(failures = state.consecutive_failures, "Jikan call failed.");
        if state.consecutive_failures >= 3 {
            state.open_until = Some(Instant::now() + self.cooldown);
            warn!("Circuit breaker opened after 3 failures.");
        }
    }

    async fn fetch_top_anime(&self) -> Result<Vec<Anime>, Status> {
        if self.should_short_circuit().await {
            return Err(Status::unavailable("Circuit breaker open for Jikan API"));
        }
        let url = format!("{}/top/anime", self.base_url);
        let response = self.client.get(url).send().await;
        self.handle_list_response(response).await
    }

    async fn search_anime(&self, query: &str) -> Result<Vec<Anime>, Status> {
        if self.should_short_circuit().await {
            return Err(Status::unavailable("Circuit breaker open for Jikan API"));
        }
        let url = format!("{}/anime", self.base_url);
        let response = self.client.get(url).query(&[("q", query)]).send().await;
        self.handle_list_response(response).await
    }

    async fn get_anime_by_id(&self, id: i32) -> Result<Anime, Status> {
        if self.should_short_circuit().await {
            return Err(Status::unavailable("Circuit breaker open for Jikan API"));
        }
        let url = format!("{}/anime/{}", self.base_url, id);
        let response = self.client.get(url).send().await;

        match response {
            Ok(r) if r.status().is_success() => match r.json::<JikanSingleResponse>().await {
                Ok(parsed) => {
                    self.record_success().await;
                    Ok(map_jikan_anime(parsed.data))
                }
                Err(e) => {
                    self.record_failure().await;
                    error!(error = %e, "Failed to parse Jikan anime by id response");
                    Err(Status::internal("Failed to parse Jikan response"))
                }
            },
            Ok(r) => {
                self.record_failure().await;
                Err(Status::unavailable(format!(
                    "Jikan returned status {}",
                    r.status()
                )))
            }
            Err(e) => {
                self.record_failure().await;
                Err(Status::unavailable(format!("Jikan request failed: {e}")))
            }
        }
    }

    async fn handle_list_response(
        &self,
        response: Result<reqwest::Response, reqwest::Error>,
    ) -> Result<Vec<Anime>, Status> {
        match response {
            Ok(r) if r.status().is_success() => match r.json::<JikanListResponse>().await {
                Ok(parsed) => {
                    self.record_success().await;
                    Ok(parsed.data.into_iter().map(map_jikan_anime).collect())
                }
                Err(e) => {
                    self.record_failure().await;
                    error!(error = %e, "Failed to parse Jikan list response");
                    Err(Status::internal("Failed to parse Jikan response"))
                }
            },
            Ok(r) => {
                self.record_failure().await;
                Err(Status::unavailable(format!(
                    "Jikan returned status {}",
                    r.status()
                )))
            }
            Err(e) => {
                self.record_failure().await;
                Err(Status::unavailable(format!("Jikan request failed: {e}")))
            }
        }
    }
}

#[tonic::async_trait]
impl QuestionsService for QuestionsSvc {
    async fn search_anime(
        &self,
        request: Request<SearchAnimeRequest>,
    ) -> Result<Response<SearchAnimeResponse>, Status> {
        let query = request.into_inner().query;
        if query.trim().is_empty() {
            return Err(Status::invalid_argument("query must not be empty"));
        }

        info!(query = %query, "SearchAnime request received");
        let animes = self.adapter.search_anime(&query).await?;
        Ok(Response::new(SearchAnimeResponse { animes }))
    }

    async fn get_anime_by_id(
        &self,
        request: Request<GetAnimeByIdRequest>,
    ) -> Result<Response<GetAnimeByIdResponse>, Status> {
        let id = request.into_inner().id;
        if id <= 0 {
            return Err(Status::invalid_argument("id must be > 0"));
        }

        info!(anime_id = id, "GetAnimeById request received");
        let anime = self.adapter.get_anime_by_id(id).await?;
        Ok(Response::new(GetAnimeByIdResponse { anime: Some(anime) }))
    }

    async fn generate_question(
        &self,
        _request: Request<GenerateQuestionRequest>,
    ) -> Result<Response<GenerateQuestionResponse>, Status> {
        info!("GenerateQuestion request received");

        match self.adapter.fetch_top_anime().await {
            Ok(mut animes) if animes.len() >= 4 => {
                animes.shuffle(&mut rand::thread_rng());
                let options = &animes[..4];
                let correct = options
                    .first()
                    .ok_or_else(|| Status::internal("No anime available for question"))?;

                let question = build_varied_question(options).unwrap_or_else(|| Question {
                    id: Uuid::new_v4().to_string(),
                    text: "Cual de estos animes tiene el titulo correcto?".to_string(),
                    option_a: options[0].title.clone(),
                    option_b: options[1].title.clone(),
                    option_c: options[2].title.clone(),
                    option_d: options[3].title.clone(),
                    correct_option: "A".to_string(),
                    anime_id: correct.id,
                });

                Ok(Response::new(GenerateQuestionResponse {
                    question: Some(question),
                }))
            }
            Ok(_) => {
                warn!("Top anime response returned less than 4 elements; using fallback question.");
                Ok(Response::new(GenerateQuestionResponse {
                    question: Some(fallback_question()),
                }))
            }
            Err(e) => {
                warn!(error = %e, "GenerateQuestion using fallback due to Jikan/circuit issue");
                Ok(Response::new(GenerateQuestionResponse {
                    question: Some(fallback_question()),
                }))
            }
        }
    }
}

fn map_jikan_anime(a: JikanAnime) -> Anime {
    Anime {
        id: a.mal_id,
        title: a.title,
        synopsis: a.synopsis.unwrap_or_default(),
        episodes: a.episodes.unwrap_or_default(),
        score: a.score.unwrap_or_default(),
        image_url: a
            .images
            .and_then(|img| img.jpg)
            .and_then(|jpg| jpg.image_url)
            .unwrap_or_default(),
    }
}

fn fallback_question() -> Question {
    Question {
        id: format!("fallback-{}", Uuid::new_v4()),
        text: "Cual de estos animes tiene el titulo correcto?".to_string(),
        option_a: "Ichigo Kurosaki".to_string(),
        option_b: "Naruto Uzumaki".to_string(),
        option_c: "Monkey D. Luffy".to_string(),
        option_d: "Eren Yeager".to_string(),
        correct_option: "B".to_string(),
        anime_id: 20,
    }
}

fn build_varied_question(options: &[Anime]) -> Option<Question> {
    if options.len() < 4 {
        return None;
    }
    let labels = ["A", "B", "C", "D"];
    let mut rng = rand::thread_rng();
    let t = rng.gen_range(0..5);
    match t {
        1 => {
            let best = options.iter().enumerate().max_by(|a, b| {
                a.1.score
                    .partial_cmp(&b.1.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })?;
            Some(Question {
                id: Uuid::new_v4().to_string(),
                text: "Cual de estos animes tiene mejor puntuacion?".to_string(),
                option_a: options[0].title.clone(),
                option_b: options[1].title.clone(),
                option_c: options[2].title.clone(),
                option_d: options[3].title.clone(),
                correct_option: labels[best.0].to_string(),
                anime_id: best.1.id,
            })
        }
        2 => {
            let best = options.iter().enumerate().max_by_key(|(_, a)| a.episodes)?;
            Some(Question {
                id: Uuid::new_v4().to_string(),
                text: "Cual de estos animes tiene mas episodios?".to_string(),
                option_a: options[0].title.clone(),
                option_b: options[1].title.clone(),
                option_c: options[2].title.clone(),
                option_d: options[3].title.clone(),
                correct_option: labels[best.0].to_string(),
                anime_id: best.1.id,
            })
        }
        3 => {
            let target = &options[0];
            let mut vals = vec![
                target.episodes.to_string(),
                options[1].episodes.to_string(),
                options[2].episodes.to_string(),
                options[3].episodes.to_string(),
            ];
            vals.sort();
            vals.dedup();
            if vals.len() < 4 {
                return None;
            }
            vals.shuffle(&mut rng);
            let correct_idx = vals
                .iter()
                .position(|v| v == &target.episodes.to_string())?;
            Some(Question {
                id: Uuid::new_v4().to_string(),
                text: format!("Cuantos episodios tiene {}?", target.title),
                option_a: vals[0].clone(),
                option_b: vals[1].clone(),
                option_c: vals[2].clone(),
                option_d: vals[3].clone(),
                correct_option: labels[correct_idx].to_string(),
                anime_id: target.id,
            })
        }
        4 => {
            let target = &options[0];
            let mut vals = vec![
                format!("{:.1}", target.score),
                format!("{:.1}", options[1].score),
                format!("{:.1}", options[2].score),
                format!("{:.1}", options[3].score),
            ];
            vals.sort();
            vals.dedup();
            if vals.len() < 4 {
                return None;
            }
            vals.shuffle(&mut rng);
            let c = format!("{:.1}", target.score);
            let correct_idx = vals.iter().position(|v| v == &c)?;
            Some(Question {
                id: Uuid::new_v4().to_string(),
                text: format!("Cual es la puntuacion aproximada de {}?", target.title),
                option_a: vals[0].clone(),
                option_b: vals[1].clone(),
                option_c: vals[2].clone(),
                option_d: vals[3].clone(),
                correct_option: labels[correct_idx].to_string(),
                anime_id: target.id,
            })
        }
        _ => Some(Question {
            id: Uuid::new_v4().to_string(),
            text: "Cual de estos animes tiene el titulo correcto?".to_string(),
            option_a: options[0].title.clone(),
            option_b: options[1].title.clone(),
            option_c: options[2].title.clone(),
            option_d: options[3].title.clone(),
            correct_option: "A".to_string(),
            anime_id: options[0].id,
        }),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let addr =
        std::env::var("QUESTIONS_SERVICE_ADDR").unwrap_or_else(|_| "0.0.0.0:50052".to_string());
    let jikan_base_url =
        std::env::var("JIKAN_BASE_URL").unwrap_or_else(|_| "https://api.jikan.moe/v4".to_string());
    let http_timeout_secs: u64 = std::env::var("HTTP_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);
    let breaker_cooldown_secs: u64 = std::env::var("CB_COOLDOWN_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);

    let adapter = JikanAdapter::new(
        jikan_base_url.clone(),
        Duration::from_secs(http_timeout_secs),
        Duration::from_secs(breaker_cooldown_secs),
    )?;

    let svc = QuestionsSvc {
        adapter: Arc::new(adapter),
    };

    info!(%addr, %jikan_base_url, http_timeout_secs, breaker_cooldown_secs, "Starting questions-service");

    Server::builder()
        .add_service(QuestionsServiceServer::new(svc))
        .serve(addr.parse()?)
        .await?;

    Ok(())
}
