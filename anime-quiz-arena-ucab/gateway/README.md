# API Gateway (Fase 8)

## Puertos
- gRPC: `0.0.0.0:50050`
- HTTP REST: `0.0.0.0:8080`

## REST endpoints
- GET `/health`
- POST `/api/users`
- POST `/api/login`
- GET `/api/anime/search?q=naruto`
- POST `/api/questions/generate`
- POST `/api/rooms`
- POST `/api/rooms/:room_id/join`
- POST `/api/rooms/:room_id/start`
- POST `/api/rooms/:room_id/answer`
- GET `/api/rooms/:room_id/leaderboard?limit=10`
- POST `/api/rooms/:room_id/end`

## CORS
Permitido: `http://localhost:5173` y `http://127.0.0.1:5173`.

## Health check
```bash
curl http://localhost:8080/health
```

## Backend
```bash
docker compose up --build users-db rooms-db redis users-service questions-service score-service game-room-service api-gateway
```

La web (carpeta `web/`) consume este REST y el gateway sigue hablando gRPC internamente con los microservicios.
