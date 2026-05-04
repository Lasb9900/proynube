# Anime Quiz Arena UCAB

Proyecto académico de arquitectura de microservicios para trivia de anime.

## Deploy en Render

Se agregó infraestructura declarativa con `render.yaml` para desplegar el sistema en **servicios separados** y usando red privada interna entre microservicios.

### 1) Pasos de despliegue

1. Sube este repositorio a GitHub.
2. En Render, crea un **Blueprint** desde el repo (Render detecta `render.yaml`).
3. Render aprovisionará automáticamente:
   - `api-gateway` (público)
   - `users-service`, `questions-service`, `game-room-service`, `score-service` (privados)
   - `web` (static site)
   - `users-db` (Postgres)
   - `rooms-db` (Postgres)
   - `score-redis` (Key Value / Redis)
4. Al finalizar, actualiza `VITE_API_BASE_URL` del servicio `web` con la URL real pública de `api-gateway` si cambia.

### 2) Servicios públicos vs privados

- **Públicos**:
  - `api-gateway` (HTTP REST en `PORT`, por defecto 8080)
  - `web` (sitio estático)
- **Privados (red interna de Render)**:
  - `users-service`
  - `questions-service`
  - `game-room-service`
  - `score-service`

### 3) Variables de entorno relevantes

- `api-gateway`
  - `PORT`
  - `GRPC_PORT`
  - `USERS_SERVICE_ADDR`
  - `QUESTIONS_SERVICE_ADDR`
  - `GAME_ROOM_SERVICE_ADDR`
  - `SCORE_SERVICE_ADDR`
- `users-service`
  - `DATABASE_URL`
- `game-room-service`
  - `DATABASE_URL`
  - `SCORE_SERVICE_ADDR`
- `score-service`
  - `REDIS_URL`
- `web`
  - `VITE_API_BASE_URL`

> Nota: las URLs de DB y Redis se inyectan desde recursos administrados en Render (`fromDatabase` / `fromService`).

### 4) Compatibilidad local

`docker-compose.yml` se mantiene sin cambios funcionales para desarrollo local.

- API Gateway local: `http://localhost:8080`
- Frontend local (Vite dev): `http://localhost:5173`

### 5) Limitaciones actuales

- El frontend usa **polling** periódico para estado de sala/ranking.
- Aún **no hay WebSockets** para eventos en tiempo real.
