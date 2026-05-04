use rand::seq::SliceRandom;
use rand::Rng;
use reqwest::Client;
use serde::Deserialize;
use std::collections::{HashMap, HashSet, VecDeque};
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
    recent_curated_by_room: Arc<Mutex<HashMap<String, VecDeque<usize>>>>,
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

struct CuratedQuestion {
    text: &'static str,
    options: [&'static str; 4],
    correct_index: usize,
    anime_id: i32,
}

const ANSWER_LABELS: [&str; 4] = ["A", "B", "C", "D"];

const CURATED_QUESTIONS: [CuratedQuestion; 60] = [
    CuratedQuestion {
        text: "Quien es el protagonista principal de Dragon Ball?",
        options: ["Goku", "Vegeta", "Naruto", "Ichigo"],
        correct_index: 0,
        anime_id: 223,
    },
    CuratedQuestion {
        text: "Cual transformacion es famosa en Dragon Ball Z?",
        options: ["Super Saiyan", "Bankai", "Gear Second", "Sharingan"],
        correct_index: 0,
        anime_id: 813,
    },
    CuratedQuestion {
        text: "Como se llama el radar para buscar esferas en Dragon Ball?",
        options: ["Dragon Radar", "Den Den Mushi", "Death Note", "Poke Radar"],
        correct_index: 0,
        anime_id: 223,
    },
    CuratedQuestion {
        text: "Quien es el principe de los Saiyajin en Dragon Ball Z?",
        options: ["Gohan", "Vegeta", "Piccolo", "Trunks"],
        correct_index: 1,
        anime_id: 813,
    },
    CuratedQuestion {
        text: "En que aldea vive Naruto al inicio de la serie?",
        options: [
            "Aldea Oculta de la Hoja",
            "Aldea Oculta de la Arena",
            "Aldea Oculta de la Niebla",
            "Aldea Oculta de la Roca",
        ],
        correct_index: 0,
        anime_id: 20,
    },
    CuratedQuestion {
        text: "Que tecnica visual pertenece al clan Uchiha?",
        options: ["Sharingan", "Bankai", "Haki", "Rasengan"],
        correct_index: 0,
        anime_id: 20,
    },
    CuratedQuestion {
        text: "Que grupo de villanos aparece en Naruto Shippuden?",
        options: ["Akatsuki", "Espada", "Cipher Pol", "Homunculos"],
        correct_index: 0,
        anime_id: 1735,
    },
    CuratedQuestion {
        text: "Cual es el sueno de Naruto?",
        options: [
            "Ser Hokage",
            "Ser Rey Pirata",
            "Ser capitan",
            "Encontrar al padre",
        ],
        correct_index: 0,
        anime_id: 20,
    },
    CuratedQuestion {
        text: "Cual es el objetivo principal de Luffy?",
        options: [
            "Convertirse en Rey de los Piratas",
            "Ser Hokage",
            "Encontrar las Esferas del Dragon",
            "Ser Shinigami",
        ],
        correct_index: 0,
        anime_id: 21,
    },
    CuratedQuestion {
        text: "Como se llama la tripulacion de Luffy?",
        options: [
            "Sombrero de Paja",
            "Akatsuki",
            "Tropa de Reconocimiento",
            "Espada",
        ],
        correct_index: 0,
        anime_id: 21,
    },
    CuratedQuestion {
        text: "Que fruta comio Luffy?",
        options: [
            "Gomu Gomu no Mi",
            "Mera Mera no Mi",
            "Ope Ope no Mi",
            "Suna Suna no Mi",
        ],
        correct_index: 0,
        anime_id: 21,
    },
    CuratedQuestion {
        text: "Quien es el espadachin principal de los Sombrero de Paja?",
        options: ["Sanji", "Zoro", "Usopp", "Franky"],
        correct_index: 1,
        anime_id: 21,
    },
    CuratedQuestion {
        text: "Que arma espiritual usan los Shinigami en Bleach?",
        options: ["Zanpakuto", "Kunai", "Death Note", "Pokeball"],
        correct_index: 0,
        anime_id: 269,
    },
    CuratedQuestion {
        text: "Como se llama la liberacion final de una Zanpakuto?",
        options: ["Bankai", "Domain Expansion", "Gear Fifth", "Nen"],
        correct_index: 0,
        anime_id: 269,
    },
    CuratedQuestion {
        text: "Quien es el protagonista de Bleach?",
        options: [
            "Ichigo Kurosaki",
            "Uryu Ishida",
            "Sosuke Aizen",
            "Yusuke Urameshi",
        ],
        correct_index: 0,
        anime_id: 269,
    },
    CuratedQuestion {
        text: "Que organizacion protege almas en Bleach?",
        options: ["Soul Society", "Akatsuki", "Survey Corps", "Magic Council"],
        correct_index: 0,
        anime_id: 269,
    },
    CuratedQuestion {
        text: "Que objeto usa Light Yagami?",
        options: ["Death Note", "Dragon Radar", "Kunai", "Zanpakuto"],
        correct_index: 0,
        anime_id: 1535,
    },
    CuratedQuestion {
        text: "Como se llama el detective rival de Light?",
        options: ["L", "Near", "Mello", "Aizawa"],
        correct_index: 0,
        anime_id: 1535,
    },
    CuratedQuestion {
        text: "Quien es el shinigami que encuentra Light?",
        options: ["Ryuk", "Rem", "Gelus", "Sidoh"],
        correct_index: 0,
        anime_id: 1535,
    },
    CuratedQuestion {
        text: "Cual era la profesion de Light al inicio de Death Note?",
        options: ["Estudiante", "Policia", "Doctor", "Fiscal"],
        correct_index: 0,
        anime_id: 1535,
    },
    CuratedQuestion {
        text: "Cual es el objetivo de Tanjiro en Demon Slayer?",
        options: [
            "Salvar a Nezuko",
            "Ser Rey Pirata",
            "Ser Hokage",
            "Atrapar criminales con una libreta",
        ],
        correct_index: 0,
        anime_id: 38000,
    },
    CuratedQuestion {
        text: "Como se llama el grupo de cazadores en Demon Slayer?",
        options: [
            "Cuerpo de Exterminio de Demonios",
            "Shinsengumi",
            "Gotei 13",
            "Guild of Mages",
        ],
        correct_index: 0,
        anime_id: 38000,
    },
    CuratedQuestion {
        text: "Que respiracion usa Tanjiro con mas frecuencia?",
        options: [
            "Respiracion del Agua",
            "Respiracion del Trueno",
            "Respiracion de la Niebla",
            "Respiracion de la Roca",
        ],
        correct_index: 0,
        anime_id: 38000,
    },
    CuratedQuestion {
        text: "Quien es la hermana de Tanjiro?",
        options: ["Nezuko", "Kanao", "Mitsuri", "Shinobu"],
        correct_index: 0,
        anime_id: 38000,
    },
    CuratedQuestion {
        text: "Contra que amenaza lucha la humanidad en Attack on Titan?",
        options: ["Titanes", "Hollows", "Demonios de la Luna", "Piratas"],
        correct_index: 0,
        anime_id: 16498,
    },
    CuratedQuestion {
        text: "Como se llama el cuerpo militar de exploracion en Attack on Titan?",
        options: ["Survey Corps", "Black Bulls", "Gotei 13", "Akatsuki"],
        correct_index: 0,
        anime_id: 16498,
    },
    CuratedQuestion {
        text: "Quien es el protagonista principal de Attack on Titan?",
        options: [
            "Eren Yeager",
            "Levi Ackerman",
            "Armin Arlert",
            "Erwin Smith",
        ],
        correct_index: 0,
        anime_id: 16498,
    },
    CuratedQuestion {
        text: "Que ciudad esta protegida por murallas en Attack on Titan?",
        options: ["Paradisis", "Shiganshina", "Westalis", "Konoha"],
        correct_index: 1,
        anime_id: 16498,
    },
    CuratedQuestion {
        text: "Que energia se usa en Jujutsu Kaisen?",
        options: ["Energia maldita", "Chakra", "Ki", "Haki"],
        correct_index: 0,
        anime_id: 40748,
    },
    CuratedQuestion {
        text: "Quien es el profesor mas famoso de Jujutsu Kaisen?",
        options: [
            "Satoru Gojo",
            "Kakashi Hatake",
            "Kisuke Urahara",
            "All Might",
        ],
        correct_index: 0,
        anime_id: 40748,
    },
    CuratedQuestion {
        text: "Que contiene el cuerpo de Yuji Itadori?",
        options: [
            "Dedos de Sukuna",
            "Nueve colas",
            "Hogyoku",
            "Piedra filosofal",
        ],
        correct_index: 0,
        anime_id: 40748,
    },
    CuratedQuestion {
        text: "Como se llama la tecnica maxima en Jujutsu Kaisen?",
        options: [
            "Domain Expansion",
            "Bankai",
            "Ultra Instinct",
            "Final Flash",
        ],
        correct_index: 0,
        anime_id: 40748,
    },
    CuratedQuestion {
        text: "Que disciplina usan Edward y Alphonse?",
        options: [
            "Alquimia",
            "Ninjutsu",
            "Magia de gremio",
            "Respiracion del agua",
        ],
        correct_index: 0,
        anime_id: 5114,
    },
    CuratedQuestion {
        text: "Que perdio Edward Elric en su transmutacion fallida?",
        options: [
            "Un brazo y una pierna",
            "La memoria",
            "El ojo derecho",
            "La voz",
        ],
        correct_index: 0,
        anime_id: 5114,
    },
    CuratedQuestion {
        text: "Como se llama el titulo estatal de Edward?",
        options: [
            "Fullmetal Alchemist",
            "White Mage",
            "Soul Reaper",
            "Hero Number One",
        ],
        correct_index: 0,
        anime_id: 5114,
    },
    CuratedQuestion {
        text: "Que buscaban los hermanos Elric?",
        options: [
            "La Piedra Filosofal",
            "One Piece",
            "Dragon Balls",
            "Death Note",
        ],
        correct_index: 0,
        anime_id: 5114,
    },
    CuratedQuestion {
        text: "Quien es el protagonista de Hunter x Hunter?",
        options: ["Gon Freecss", "Killua Zoldyck", "Kurapika", "Leorio"],
        correct_index: 0,
        anime_id: 11061,
    },
    CuratedQuestion {
        text: "Que energia se utiliza en Hunter x Hunter?",
        options: ["Nen", "Chakra", "Reiatsu", "Mana"],
        correct_index: 0,
        anime_id: 11061,
    },
    CuratedQuestion {
        text: "Cual es la profesion objetivo de Gon?",
        options: ["Hunter", "Ninja", "Pirata", "Alquimista"],
        correct_index: 0,
        anime_id: 11061,
    },
    CuratedQuestion {
        text: "De que familia viene Killua?",
        options: ["Zoldyck", "Uchiha", "Elric", "Jaeger"],
        correct_index: 0,
        anime_id: 11061,
    },
    CuratedQuestion {
        text: "Quien es el protagonista de My Hero Academia?",
        options: [
            "Izuku Midoriya",
            "Katsuki Bakugo",
            "Shoto Todoroki",
            "Tenya Iida",
        ],
        correct_index: 0,
        anime_id: 31964,
    },
    CuratedQuestion {
        text: "Como se llama el heroe simbolo en My Hero Academia?",
        options: ["All Might", "Endeavor", "Eraser Head", "Mirko"],
        correct_index: 0,
        anime_id: 31964,
    },
    CuratedQuestion {
        text: "Que nombre tiene el poder heredado de Deku?",
        options: [
            "One For All",
            "All For One",
            "Full Cowling",
            "Detroit Smash",
        ],
        correct_index: 0,
        anime_id: 31964,
    },
    CuratedQuestion {
        text: "En que academia estudian los heroes?",
        options: [
            "U.A.",
            "Shuchiin",
            "Tokyo Jujutsu High",
            "Shinigami Academy",
        ],
        correct_index: 0,
        anime_id: 31964,
    },
    CuratedQuestion {
        text: "Quien es el protagonista de One Punch Man?",
        options: ["Saitama", "Genos", "King", "Mumen Rider"],
        correct_index: 0,
        anime_id: 30276,
    },
    CuratedQuestion {
        text: "Cual es el rasgo principal de Saitama?",
        options: [
            "Derrota enemigos de un golpe",
            "Controla titanes",
            "Usa un cuaderno mortal",
            "Es rey de piratas",
        ],
        correct_index: 0,
        anime_id: 30276,
    },
    CuratedQuestion {
        text: "Que cyborg acompana a Saitama?",
        options: ["Genos", "Drive Knight", "Metal Bat", "Bang"],
        correct_index: 0,
        anime_id: 30276,
    },
    CuratedQuestion {
        text: "Que asociacion clasifica heroes en One Punch Man?",
        options: [
            "Hero Association",
            "Magic Council",
            "Akatsuki",
            "Soul Society",
        ],
        correct_index: 0,
        anime_id: 30276,
    },
    CuratedQuestion {
        text: "Quien queda atrapado en SAO?",
        options: ["Kirito", "Asuna", "Sinon", "Leafa"],
        correct_index: 0,
        anime_id: 11757,
    },
    CuratedQuestion {
        text: "Como se llama el juego de Sword Art Online?",
        options: [
            "Sword Art Online",
            "Aincrad Storm",
            "Gun Gale",
            "Ordinal Scale",
        ],
        correct_index: 0,
        anime_id: 11757,
    },
    CuratedQuestion {
        text: "Que apodo recibe Kirito?",
        options: [
            "Espadachin Negro",
            "Heroe numero uno",
            "Cazador blanco",
            "Shinigami sustituto",
        ],
        correct_index: 0,
        anime_id: 11757,
    },
    CuratedQuestion {
        text: "Que pasa si mueres dentro de SAO en el primer arco?",
        options: [
            "Mueres en la vida real",
            "Reapareces en ciudad",
            "Pierdes oro",
            "Reinicias nivel",
        ],
        correct_index: 0,
        anime_id: 11757,
    },
    CuratedQuestion {
        text: "Quien es el protagonista de Tokyo Ghoul?",
        options: ["Ken Kaneki", "Touka Kirishima", "Hide", "Arima"],
        correct_index: 0,
        anime_id: 22319,
    },
    CuratedQuestion {
        text: "Que necesita comer un ghoul para sobrevivir?",
        options: [
            "Carne humana",
            "Semillas especiales",
            "Sangre de titan",
            "Fruta del diablo",
        ],
        correct_index: 0,
        anime_id: 22319,
    },
    CuratedQuestion {
        text: "Que organizacion investiga ghouls en Tokyo Ghoul?",
        options: ["CCG", "CID", "Gotei 13", "Survey Corps"],
        correct_index: 0,
        anime_id: 22319,
    },
    CuratedQuestion {
        text: "Que simboliza la mascara de Kaneki en combate?",
        options: [
            "Su identidad ghoul",
            "Su rango militar",
            "Su clan ninja",
            "Su gremio",
        ],
        correct_index: 0,
        anime_id: 22319,
    },
    CuratedQuestion {
        text: "Quien es el protagonista de Fairy Tail?",
        options: [
            "Natsu Dragneel",
            "Gray Fullbuster",
            "Erza Scarlet",
            "Laxus Dreyar",
        ],
        correct_index: 0,
        anime_id: 6702,
    },
    CuratedQuestion {
        text: "Como se llama el gremio principal de Fairy Tail?",
        options: ["Fairy Tail", "Black Bulls", "Blue Pegasus", "Lamia Scale"],
        correct_index: 0,
        anime_id: 6702,
    },
    CuratedQuestion {
        text: "Que magia usa principalmente Natsu?",
        options: [
            "Dragon Slayer de fuego",
            "Magia de cartas",
            "Respiracion solar",
            "Alquimia",
        ],
        correct_index: 0,
        anime_id: 6702,
    },
    CuratedQuestion {
        text: "Que companera celestial acompana a Natsu?",
        options: [
            "Lucy Heartfilia",
            "Juvia Lockser",
            "Levy McGarden",
            "Wendy Marvell",
        ],
        correct_index: 0,
        anime_id: 6702,
    },
];

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
        request: Request<GenerateQuestionRequest>,
    ) -> Result<Response<GenerateQuestionResponse>, Status> {
        let req = request.into_inner();
        let room_id = if req.room_id.trim().is_empty() {
            "global-demo-room".to_string()
        } else {
            req.room_id
        };

        info!(room_id = %room_id, "GenerateQuestion request received");

        let use_curated = rand::thread_rng().gen_bool(0.75);

        if use_curated {
            if let Some((question, index)) =
                generate_curated_question(&room_id, &self.recent_curated_by_room).await
            {
                info!(index, "GenerateQuestion using curated pool index={index}");
                return Ok(Response::new(GenerateQuestionResponse {
                    question: Some(question),
                }));
            }
            warn!("Curated pool unavailable; falling back to dynamic template");
        }

        info!("GenerateQuestion using dynamic template");
        match self.adapter.fetch_top_anime().await {
            Ok(mut animes) if animes.len() >= 4 => {
                animes.shuffle(&mut rand::thread_rng());
                let options = &animes[..4];
                let correct = options
                    .first()
                    .ok_or_else(|| Status::internal("No anime available for question"))?;

                let question = build_varied_question(options).unwrap_or_else(|| {
    let correct_index = options
        .iter()
        .position(|anime| anime.id == correct.id)
        .unwrap_or(0);

    Question {
        id: Uuid::new_v4().to_string(),
        text: "Cual de estos titulos corresponde a un anime real?".to_string(),
        option_a: options[0].title.clone(),
        option_b: options[1].title.clone(),
        option_c: options[2].title.clone(),
        option_d: options[3].title.clone(),
        correct_option: ANSWER_LABELS[correct_index].to_string(),
        anime_id: correct.id,
    }
});

                Ok(Response::new(GenerateQuestionResponse {
                    question: Some(question),
                }))
            }
            Ok(_) => {
                warn!("Top anime response returned less than 4 elements; using curated fallback");
                let question = generate_curated_question(&room_id, &self.recent_curated_by_room)
                    .await
                    .map(|(question, _)| question)
                    .unwrap_or_else(fallback_question);
                Ok(Response::new(GenerateQuestionResponse {
                    question: Some(question),
                }))
            }
            Err(e) => {
                warn!(error = %e, "Dynamic generation failed; using curated fallback");
                let question = generate_curated_question(&room_id, &self.recent_curated_by_room)
                    .await
                    .map(|(question, _)| question)
                    .unwrap_or_else(fallback_question);
                Ok(Response::new(GenerateQuestionResponse {
                    question: Some(question),
                }))
            }
        }
    }
}

async fn generate_curated_question(
    room_id: &str,
    recent_curated_by_room: &Arc<Mutex<HashMap<String, VecDeque<usize>>>>,
) -> Option<(Question, usize)> {
    let selected_index = {
        let mut recent = recent_curated_by_room.lock().await;
        let room_recent = recent.entry(room_id.to_string()).or_default();

        let mut chosen = rand::thread_rng().gen_range(0..CURATED_QUESTIONS.len());
        for _ in 0..10 {
            let candidate = rand::thread_rng().gen_range(0..CURATED_QUESTIONS.len());
            if !room_recent.contains(&candidate) {
                chosen = candidate;
                break;
            }
            chosen = candidate;
        }

        room_recent.push_back(chosen);
        while room_recent.len() > 5 {
            room_recent.pop_front();
        }

        chosen
    };
    let curated = &CURATED_QUESTIONS[selected_index];

    if curated.correct_index >= curated.options.len() {
        return None;
    }
    if curated.options.iter().any(|opt| opt.trim().is_empty()) {
        return None;
    }

    let mut seen = HashSet::new();
    if !curated.options.iter().all(|opt| seen.insert(*opt)) {
        return None;
    }

    let mut indexed: Vec<(usize, &str)> = curated.options.iter().copied().enumerate().collect();
    indexed.shuffle(&mut rand::thread_rng());
    let correct_pos = indexed
        .iter()
        .position(|(idx, _)| *idx == curated.correct_index)?;

    let question = Question {
        id: Uuid::new_v4().to_string(),
        text: curated.text.to_string(),
        option_a: indexed[0].1.to_string(),
        option_b: indexed[1].1.to_string(),
        option_c: indexed[2].1.to_string(),
        option_d: indexed[3].1.to_string(),
        correct_option: ANSWER_LABELS[correct_pos].to_string(),
        anime_id: curated.anime_id,
    };
    info!(question_id = %question.id, index = selected_index, "Curated question generated question_id=... index=...");
    Some((question, selected_index))
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
        text: "Quien es el protagonista de Naruto?".to_string(),
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
    text: "Cual de estos titulos corresponde a un anime real?".to_string(),
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
        recent_curated_by_room: Arc::new(Mutex::new(HashMap::new())),
    };

    info!(%addr, %jikan_base_url, http_timeout_secs, breaker_cooldown_secs, "Starting questions-service");

    Server::builder()
        .add_service(QuestionsServiceServer::new(svc))
        .serve(addr.parse()?)
        .await?;

    Ok(())
}
