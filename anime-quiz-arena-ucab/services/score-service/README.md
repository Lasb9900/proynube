# score-service

Servicio gRPC encargado de gestionar puntajes por sala y leaderboard en **Anime Quiz Arena UCAB**.

## Puerto
- gRPC: `50054`

## Variables de entorno
- `REDIS_URL` (default recomendado en compose): `redis://redis:6379`

## Persistencia en Redis (Sorted Sets)
El servicio usa **Redis Sorted Sets** para ranking por sala.

- Key: `leaderboard:{room_id}`
- Member: `user_id`
- Score: `points`

Esto permite:
- incrementar puntos en O(log N) con `ZINCRBY`
- consultar top N en orden descendente con `ZREVRANGE WITHSCORES`

## Métodos gRPC

### `AddScore`
- Entrada: `room_id`, `user_id`, `points`
- Acción: incrementa el score del usuario en el sorted set de la sala.
- Salida: `user_id` y `total_points` (nuevo acumulado).

### `GetScore`
- Entrada: `room_id`, `user_id`
- Acción: lee el score actual del usuario con `ZSCORE`.
- Salida: `points` (si no existe devuelve `0`).

### `GetLeaderboard`
- Entrada: `room_id`, `limit`
- Acción: obtiene top N de la sala en orden descendente.
- Regla: si `limit <= 0`, usa `10` por defecto.
- Salida: lista `entries` con `user_id`, `points`, `rank`.

### `WatchLeaderboard` (Server Streaming)
- Entrada: `room_id`, `limit`
- Acción: emite el leaderboard cada 2 segundos.
- Demo: envía 10 iteraciones.

## Ejecutar con Docker
Desde la raíz del proyecto (`anime-quiz-arena-ucab`):

```bash
docker compose up --build redis score-service
```

Logs esperados:
- `Starting score-service on 0.0.0.0:50054`
- `Connected to Redis`

## Cómo probar
Opciones simples:
1. Usar un cliente gRPC (grpcurl, BloomRPC, Postman gRPC).
2. Probar llamadas en este orden:
   - `AddScore(room_id="shonen-battle", user_id="lasb", points=100)`
   - `AddScore(room_id="shonen-battle", user_id="lasb", points=50)`
   - `GetScore(room_id="shonen-battle", user_id="lasb")` => `150`
   - `AddScore(room_id="shonen-battle", user_id="player2", points=80)`
   - `GetLeaderboard(room_id="shonen-battle", limit=10)`
   - `WatchLeaderboard(room_id="shonen-battle", limit=10)`

## Patrones aplicados
- **Database per Service**: score-service usa Redis dedicado a su contexto de puntajes.
- **Leaderboard con Redis Sorted Sets**.
- **Server Streaming** para actualizaciones periódicas con `WatchLeaderboard`.
