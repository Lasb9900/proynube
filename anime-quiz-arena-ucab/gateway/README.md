# API Gateway (Fase 7)

Gateway gRPC del proyecto **Anime Quiz Arena UCAB**. Actúa como punto único de entrada para clientes y delega llamadas a servicios internos.

## Puerto
- gRPC: `50050`

## Variables de entorno
- `USERS_SERVICE_ADDR` (default: `users-service:50051`)
- `QUESTIONS_SERVICE_ADDR` (default: `questions-service:50052`)
- `GAME_ROOM_SERVICE_ADDR` (default: `game-room-service:50053`)
- `SCORE_SERVICE_ADDR` (default: `score-service:50054`)
- Opcional: `RUST_LOG=info`

## Servicios internos
- `users-service`
- `questions-service`
- `game-room-service`
- `score-service`

## Métodos expuestos
- `CreateUser`
- `LoginBasic`
- `SearchAnime`
- `GenerateQuestion`
- `CreateRoom`
- `JoinRoom`
- `StartGame`
- `SubmitAnswer`
- `GetLeaderboard`
- `EndGame`

## Levantar con Docker Compose
```bash
docker compose up --build users-db rooms-db redis users-service questions-service score-service game-room-service api-gateway
```

## Pruebas rápidas con grpcurl
Desde la raíz del repo:

```bash
grpcurl -plaintext localhost:50050 list
grpcurl -plaintext localhost:50050 list animequiz.gateway.v1.GatewayService
```

### Flujo demo
1) CreateUser
```bash
grpcurl -plaintext -d '{"username":"gateway-user","email":"gateway-user@ucab.edu.ve","password":"123456"}' \
localhost:50050 animequiz.gateway.v1.GatewayService/CreateUser
```

2) LoginBasic
```bash
grpcurl -plaintext -d '{"email":"gateway-user@ucab.edu.ve","password":"123456"}' \
localhost:50050 animequiz.gateway.v1.GatewayService/LoginBasic
```

3) GenerateQuestion
```bash
grpcurl -plaintext -d '{"room_id":"room-1"}' \
localhost:50050 animequiz.gateway.v1.GatewayService/GenerateQuestion
```

4) CreateRoom
```bash
grpcurl -plaintext -d '{"name":"Gateway Shonen Battle","created_by":"<USER_ID>"}' \
localhost:50050 animequiz.gateway.v1.GatewayService/CreateRoom
```

5) JoinRoom
```bash
grpcurl -plaintext -d '{"room_id":"<ROOM_ID>","user_id":"<USER_ID>"}' \
localhost:50050 animequiz.gateway.v1.GatewayService/JoinRoom
```

6) StartGame
```bash
grpcurl -plaintext -d '{"room_id":"<ROOM_ID>"}' \
localhost:50050 animequiz.gateway.v1.GatewayService/StartGame
```

7) SubmitAnswer correcta
```bash
grpcurl -plaintext -d '{"room_id":"<ROOM_ID>","user_id":"<USER_ID>","question_id":"q1","selected_option":"B","correct_option":"B"}' \
localhost:50050 animequiz.gateway.v1.GatewayService/SubmitAnswer
```

8) GetLeaderboard
```bash
grpcurl -plaintext -d '{"room_id":"<ROOM_ID>","limit":10}' \
localhost:50050 animequiz.gateway.v1.GatewayService/GetLeaderboard
```

9) EndGame
```bash
grpcurl -plaintext -d '{"room_id":"<ROOM_ID>"}' \
localhost:50050 animequiz.gateway.v1.GatewayService/EndGame
```

## Patrones aplicados
- **API Gateway**: un punto único para operaciones de usuarios, preguntas, rooms y score.
- **Service Discovery** por nombres de Docker Compose (DNS interno).
- **Circuit Breaker básico / error controlado**: ante falla de conexión, devuelve `UNAVAILABLE` y registra logs.
- **Service-to-service communication** por gRPC.
