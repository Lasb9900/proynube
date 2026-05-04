# live-events-service

Servicio gRPC para manejar eventos en tiempo real del juego usando **bidirectional streaming**.

## Puerto
- gRPC: `50055`

## Bidirectional streaming
`GameStream` permite que:
- el cliente envíe eventos continuamente al servidor,
- y el servidor responda confirmaciones en el mismo canal mientras el stream siga abierto.

Esto se implementa con `tokio::sync::mpsc` para emitir respuestas del servidor.

## Método principal: `GameStream`
- Tipo: `rpc GameStream(stream GameStreamRequest) returns (stream GameStreamResponse)`
- Por cada evento recibido:
  1. valida `room_id` y `event_type`,
  2. registra logs con `tracing`,
  3. envía una confirmación al cliente.

### Eventos soportados y respuesta
- `PlayerJoined` -> `Player joined room`
- `QuestionStarted` -> `Question started`
- `AnswerSubmitted` -> `Answer received`
- `ScoreUpdated` -> `Score updated`
- `GameFinished` -> `Game finished`
- Otros -> `Unknown event type` (sin cerrar el stream)

## Ejecutar con Docker
Desde la raíz del proyecto (`anime-quiz-arena-ucab`):

```bash
docker compose up --build live-events-service
```

Log esperado al iniciar:
- `Starting live-events-service on 0.0.0.0:50055`

## Cómo probar
Puedes probar con `grpcurl` (si tienes reflection desactivada, usa `-import-path` y `-proto`), o con un cliente simple que abra `GameStream` y envíe eventos consecutivos.

Ejemplo de eventos a enviar:
1. `PlayerJoined`
2. `QuestionStarted`
3. `AnswerSubmitted`
4. `ScoreUpdated`
5. `GameFinished`

## Patrones aplicados
- **Event-Driven Architecture**
- **Bidirectional Streaming**
