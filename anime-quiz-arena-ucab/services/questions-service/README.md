# questions-service

Servicio gRPC en Rust (`tonic` + `tokio`) que adapta la API REST de Jikan a contratos internos gRPC.

## Variables de entorno

- `QUESTIONS_SERVICE_ADDR` (default: `0.0.0.0:50052`)
- `JIKAN_BASE_URL` (default: `https://api.jikan.moe/v4`)
- `HTTP_TIMEOUT_SECS` (default: `5`)
- `CB_COOLDOWN_SECS` (default: `30`)

## Generacion de preguntas para demo

`GenerateQuestion` ahora combina dos fuentes:

- **Banco curado interno** (aprox. 60%): preguntas reconocibles y variadas de series populares.
- **Plantillas dinamicas con Jikan** (aprox. 40%): preguntas generadas con datos en tiempo real.

Si falla Jikan (timeout, circuito abierto, etc.), el servicio hace fallback a preguntas curadas.
Si en algun caso el banco curado no estuviera disponible, mantiene fallback local para estabilidad.

El banco curado:

- No usa imagenes ni assets.
- Mantiene 4 opciones por pregunta.
- Mezcla opciones en cada generacion para que la correcta no quede fija.
- Esta orientado a demo (preguntas faciles/medias, series populares).

## Ejecutar local

```bash
cd anime-quiz-arena-ucab/services/questions-service
cargo run
```

## Ejecutar con Docker Compose

```bash
cd anime-quiz-arena-ucab
docker compose build questions-service
docker compose up questions-service
```

## Ejemplos de uso (grpcurl)

> Requiere `grpcurl` instalado.

### SearchAnime

```bash
grpcurl -plaintext -d '{"query":"Naruto"}' localhost:50052 animequiz.questions.v1.QuestionsService/SearchAnime
```

### GetAnimeById

```bash
grpcurl -plaintext -d '{"id":20}' localhost:50052 animequiz.questions.v1.QuestionsService/GetAnimeById
```

### GenerateQuestion

```bash
grpcurl -plaintext -d '{"room_id":"demo-room"}' localhost:50052 animequiz.questions.v1.QuestionsService/GenerateQuestion
```

## External API Adapter

Este servicio implementa el patron **External API Adapter** porque expone gRPC estable hacia adentro del sistema, mientras consume REST/JSON de Jikan (`/anime`, `/anime/{id}`, `/top/anime`) hacia afuera. Asi, los demas microservicios nunca dependen directamente del formato externo.

## Circuit Breaker basico

- Se registran fallos consecutivos al llamar Jikan.
- Al llegar a **3 fallos seguidos**, el circuito se abre por `CB_COOLDOWN_SECS`.
- Mientras esta abierto, `GenerateQuestion` responde usando fallback interno sin invocar Jikan.
- Al vencer el cooldown, se permite intentar de nuevo (half-open simplificado).
