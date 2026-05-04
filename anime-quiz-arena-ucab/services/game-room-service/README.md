# game-room-service

Servicio gRPC encargado de gestionar salas de juego de trivia para Anime Quiz Arena UCAB.

## Puerto
- gRPC: `50053`

## Variables de entorno
- `DATABASE_URL` (default: `postgres://postgres:postgres@rooms-db:5432/rooms_db`)
- `SCORE_SERVICE_ADDR` (default: `score-service:50054`)

## Tablas PostgreSQL
Se crean automáticamente al iniciar el servicio:

- `rooms`
  - `id UUID PRIMARY KEY`
  - `name VARCHAR NOT NULL`
  - `status VARCHAR NOT NULL` (`WAITING`, `STARTED`, `FINISHED`)
  - `created_by UUID NOT NULL`
  - `created_at TIMESTAMP NOT NULL`

- `room_players`
  - `room_id UUID NOT NULL`
  - `user_id UUID NOT NULL`
  - `joined_at TIMESTAMP NOT NULL`
  - `PRIMARY KEY (room_id, user_id)`

## Métodos gRPC
- `CreateRoom`
- `JoinRoom`
- `StartGame`
- `EndGame`
- `SubmitAnswer`

## Integración con score-service
En `SubmitAnswer`, si la respuesta es correcta (`selected_option == correct_option`), el servicio llama `AddScore` con `points=100`.

Si score-service falla:
- No se cae game-room-service.
- Se responde `correct=true`, `points_awarded=100`, `total_points=0` y mensaje de degradación controlada.

## Ejecutar con Docker Compose
```bash
docker compose up --build rooms-db redis score-service game-room-service
```

## Pruebas con grpcurl
> Ajusta UUIDs/room_id según tus respuestas.

```bash
# 1) CreateRoom
grpcurl -plaintext -d '{"name":"Shonen Battle","created_by":"6ccc596d-0d29-4177-b07f-e02cd6e61c6c"}' localhost:50053 animequiz.gameroom.v1.GameRoomService/CreateRoom

# 2) JoinRoom
grpcurl -plaintext -d '{"room_id":"<ROOM_ID>","user_id":"6ccc596d-0d29-4177-b07f-e02cd6e61c6c"}' localhost:50053 animequiz.gameroom.v1.GameRoomService/JoinRoom

# 3) StartGame
grpcurl -plaintext -d '{"room_id":"<ROOM_ID>"}' localhost:50053 animequiz.gameroom.v1.GameRoomService/StartGame

# 4) SubmitAnswer correcta
grpcurl -plaintext -d '{"room_id":"<ROOM_ID>","user_id":"6ccc596d-0d29-4177-b07f-e02cd6e61c6c","question_id":"q1","selected_option":"B","correct_option":"B"}' localhost:50053 animequiz.gameroom.v1.GameRoomService/SubmitAnswer

# 5) SubmitAnswer incorrecta
grpcurl -plaintext -d '{"room_id":"<ROOM_ID>","user_id":"6ccc596d-0d29-4177-b07f-e02cd6e61c6c","question_id":"q1","selected_option":"A","correct_option":"B"}' localhost:50053 animequiz.gameroom.v1.GameRoomService/SubmitAnswer

# 6) EndGame
grpcurl -plaintext -d '{"room_id":"<ROOM_ID>"}' localhost:50053 animequiz.gameroom.v1.GameRoomService/EndGame

# 7) SubmitAnswer después de EndGame (debe fallar failed_precondition)
grpcurl -plaintext -d '{"room_id":"<ROOM_ID>","user_id":"6ccc596d-0d29-4177-b07f-e02cd6e61c6c","question_id":"q1","selected_option":"B","correct_option":"B"}' localhost:50053 animequiz.gameroom.v1.GameRoomService/SubmitAnswer
```

## Patrones aplicados
- Microservices
- Database per Service
- Service-to-service communication (gRPC)
- Saga/compensación simple en `SubmitAnswer + AddScore`
- Circuit breaker básico por degradación controlada cuando score-service falla
