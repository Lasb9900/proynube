# Anime Quiz Arena UCAB

Anime Quiz Arena es una aplicación distribuida de trivia multijugador sobre anime, desarrollada como proyecto de Computación en la Nube / Sistemas Distribuidos.

El sistema permite crear usuarios, crear salas, unirse a partidas multijugador, generar preguntas, responderlas en tiempo real y consultar un ranking de puntajes. La arquitectura está basada en microservicios en Rust, comunicación gRPC interna, API Gateway HTTP, bases de datos separadas y frontend React.

---

## Tabla de contenido

1. Descripción general
2. Arquitectura
3. Tecnologías utilizadas
4. Microservicios
5. Bases de datos
6. Comunicación RPC
7. Patrones de diseño aplicados
8. Estructura del proyecto
9. Requisitos previos
10. Ejecución local
11. Ejecución del frontend
12. Pruebas multijugador en LAN
13. Pruebas remotas con ZeroTier
14. Flujo de juego
15. Endpoints principales del API Gateway
16. Pruebas backend desde PowerShell
17. Comandos útiles de Docker
18. Errores comunes
19. Cumplimiento académico
20. Estado actual del proyecto

---

# 1. Descripción general

Anime Quiz Arena es un juego de trivia multijugador donde varios jugadores ingresan a una misma sala y compiten respondiendo preguntas de selección múltiple.

El sistema está diseñado para demostrar conceptos de computación en la nube y sistemas distribuidos:

- Microservicios.
- Contenedores.
- API Gateway.
- Comunicación gRPC.
- Múltiples bases de datos.
- Separación de responsabilidades.
- Multijugador en red.
- Integración con API externa.
- Patrones de diseño.
- Frontend cliente interactivo.

---

# 2. Arquitectura

La arquitectura general es:

```txt
Frontend React/Vite
        |
        | HTTP
        v
API Gateway
        |
        | gRPC interno
        v
Microservicios Rust
        |
        v
PostgreSQL / Redis / Jikan API



El frontend no se comunica directamente con cada microservicio. Todas las peticiones pasan por el API Gateway, que traduce las llamadas HTTP hacia llamadas gRPC internas.

3. Tecnologías utilizadas
   Backend
   Rust
   Tonic gRPC
   Axum
   Tokio
   SQLx
   Redis
   Reqwest
   Serde
   Tracing
   Frontend
   React
   Vite
   TypeScript
   CSS
   Sonner
   Framer Motion
   Canvas Confetti
   Lucide React
   Infraestructura local
   Docker
   Docker Compose
   PostgreSQL
   Redis
   ZeroTier
   API externa
   Jikan API

Jikan se utiliza para obtener información de anime y complementar la generación dinámica de preguntas.

4. Microservicios
   4.1 API Gateway

El API Gateway es el punto de entrada principal del sistema.

Expone endpoints HTTP para el frontend y se comunica con los demás servicios usando gRPC.

Puertos:

HTTP: 8080
gRPC: 50050

Responsabilidades:

Recibir peticiones HTTP del frontend.
Llamar a microservicios internos por gRPC.
Normalizar respuestas.
Manejar errores.
Exponer health check.
Centralizar acceso al sistema.
4.2 Users Service

Servicio responsable de usuarios.

Puerto:

gRPC: 50051

Base de datos:

PostgreSQL users-db

Responsabilidades:

Crear usuarios.
Validar login.
Almacenar datos de usuario.
Generar identificadores UUID.
4.3 Questions Service

Servicio responsable de preguntas.

Puerto:

gRPC: 50052

Dependencias:

Jikan API
Preguntas curadas locales

Responsabilidades:

Buscar anime.
Consultar anime por ID.
Generar preguntas.
Mantener una pregunta activa por sala.
Permitir que el host fuerce una nueva pregunta.
Evitar que jugadores no-host generen preguntas accidentalmente.

Regla importante:

force_new=false
Solo consulta la pregunta activa.
Si no existe pregunta activa, devuelve question: null.

force_new=true
Genera una nueva pregunta.
La guarda como pregunta activa de la sala.
La devuelve a los jugadores.

Esto garantiza que todos los jugadores de una sala reciban el mismo question_id.

4.4 Game Room Service

Servicio responsable de salas y estado de partida.

Puerto:

gRPC: 50053

Base de datos:

PostgreSQL rooms-db

Responsabilidades:

Crear salas.
Unir jugadores.
Iniciar partida.
Finalizar partida.
Registrar respuestas.
Consultar estado de sala.
Contar jugadores que respondieron una pregunta.

El conteo de respuestas se realiza usando:

room_id + question_id

Por eso es importante que todos los jugadores respondan el mismo question_id.

4.5 Score Service

Servicio responsable de puntajes.

Puerto:

gRPC: 50054

Base de datos:

Redis

Responsabilidades:

Sumar puntajes.
Guardar puntuaciones por sala.
Consultar leaderboard.
Ordenar jugadores por ranking.
4.6 Live Events Service

Servicio preparado para eventos en vivo mediante streaming gRPC.

Puerto:

gRPC: 50055

Responsabilidades:

Demostrar streaming bidireccional.
Permitir eventos en tiempo real.
Soportar comunicación estilo pub/sub.

Actualmente el flujo principal del frontend funciona mediante polling, pero este servicio permite demostrar streaming bidireccional para los requisitos académicos.

5. Bases de datos

El proyecto usa múltiples tecnologías de almacenamiento.

PostgreSQL

Se usa para datos estructurados y persistentes.

users-db:
usuarios

rooms-db:
salas
jugadores en sala
respuestas
Redis

Se usa para datos de acceso rápido.

score-service:
puntajes
leaderboard

Esto permite cumplir el requisito de múltiples tecnologías de base de datos:

SQL: PostgreSQL
Clave-valor: Redis 6. Comunicación RPC

El proyecto utiliza gRPC para la comunicación interna entre servicios.

RPC unaria / síncrona

Ejemplos:

Crear usuario.
Login.
Crear sala.
Unirse a sala.
Iniciar partida.
Enviar respuesta.
Consultar leaderboard.
Consultar estado de sala.
RPC asíncrona

Ejemplos:

Generación de preguntas.
Actualización de puntajes.
Consulta periódica del estado de la sala.
Streaming bidireccional

Ejemplo:

live-events-service

Este servicio permite demostrar comunicación continua entre cliente y servidor.

7. Patrones de diseño aplicados
   7.1 API Gateway

El frontend se comunica con un único punto de entrada.

Ventajas:

Oculta microservicios internos.
Centraliza rutas.
Facilita cambios internos.
Simplifica el cliente.
7.2 Database per Service

Cada servicio administra sus propios datos.

Ejemplos:

users-service usa users-db.
game-room-service usa rooms-db.
score-service usa Redis.
7.3 Adapter

questions-service usa un adaptador para consumir Jikan API.

Ventajas:

Aísla la API externa.
Permite cambiar proveedor.
Normaliza datos externos al modelo interno.
7.4 Circuit Breaker

questions-service protege llamadas a Jikan.

Si la API externa falla repetidamente, se abre el circuito temporalmente para evitar sobrecargar el sistema.

7.5 DTO / Mapper

El API Gateway transforma mensajes gRPC en respuestas HTTP entendibles por el frontend.

Ejemplos:

User -> UserDto
Room -> RoomDto
Question -> QuestionDto
ScoreEntry -> ScoreEntryDto
7.6 Service Layer

Cada microservicio encapsula una responsabilidad de negocio clara:

Usuarios.
Preguntas.
Salas.
Puntajes.
Eventos.
7.7 Event Streaming / Pub-Sub

live-events-service permite demostrar eventos en vivo mediante streaming gRPC.

8. Estructura del proyecto
   anime-quiz-arena-ucab/
   │
   ├── gateway/
   │ ├── src/
   │ ├── proto/
   │ ├── Cargo.toml
   │ └── Dockerfile
   │
   ├── services/
   │ ├── users-service/
   │ ├── questions-service/
   │ ├── game-room-service/
   │ ├── score-service/
   │ └── live-events-service/
   │
   ├── web/
   │ ├── src/
   │ ├── package.json
   │ ├── vite.config.ts
   │ └── .env.local
   │
   ├── proto/
   │ ├── users.proto
   │ ├── questions.proto
   │ ├── gameroom.proto
   │ ├── score.proto
   │ ├── gateway.proto
   │ └── liveevents.proto
   │
   ├── docker-compose.yml
   └── README.md
9. Requisitos previos

Para ejecutar el proyecto localmente:

Docker Desktop
Docker Compose
Node.js
npm
Git
PowerShell

Para pruebas remotas:

ZeroTier 10. Ejecución local del backend

Desde la raíz del proyecto:

cd C:\work\Githubdesk\proynube\anime-quiz-arena-ucab
docker compose up --build

Para reconstrucción limpia:

docker compose down
docker compose build --no-cache
docker compose up --build

Para reconstruir solo servicios específicos:

docker compose down
docker compose build --no-cache api-gateway questions-service
docker compose up --build 11. Ejecución del frontend

En otra terminal:

cd C:\work\Githubdesk\proynube\anime-quiz-arena-ucab\web
npm install
npm run dev -- --host 0.0.0.0

El frontend queda disponible en:

http://localhost:5173

o en algunos casos Vite puede usar:

http://localhost:5174

si el puerto 5173 está ocupado.

12. Configuración del frontend

El archivo web/.env.local debe quedar así:

VITE_API_BASE_URL=

La razón es que se usa el proxy de Vite.

En vite.config.ts debe existir algo similar a:

import { defineConfig } from "vite";

export default defineConfig({
server: {
port: 5173,
host: true,
proxy: {
"/api": {
target: "http://localhost:8080",
changeOrigin: true,
},
"/health": {
target: "http://localhost:8080",
changeOrigin: true,
},
},
},
});

Esto evita problemas de CORS durante las pruebas locales.

13. Pruebas multijugador en LAN

Para probar desde otros dispositivos en la misma red WiFi:

Levantar backend:
docker compose up --build
Levantar frontend:
npm run dev -- --host 0.0.0.0
Obtener IP local con:
ipconfig

Ejemplo:

192.168.1.91
Los demás dispositivos entran a:
http://192.168.1.91:5173 14. Pruebas remotas con ZeroTier

También se puede probar con jugadores fuera de la misma red usando ZeroTier.

Ejemplo de IP ZeroTier del host:

10.103.39.100

Los jugadores deben:

Instalar ZeroTier.
Unirse a la red ZeroTier.
Ser autorizados desde el panel.
Entrar al frontend usando la IP ZeroTier del host.

URL de acceso:

http://10.103.39.100:5173

Como el frontend usa proxy de Vite, las peticiones /api/... se redirigen al API Gateway local del host.

15. Flujo de juego

El flujo normal es:

1. Usuario inicia sesión o se registra.
2. Host crea una sala.
3. Otros jugadores se unen usando el ID de sala.
4. Host inicia la partida.
5. Host genera la primera pregunta.
6. Todos los jugadores reciben la misma pregunta.
7. Cada jugador responde.
8. El sistema actualiza respuestas y puntajes.
9. Cuando todos responden, el host genera la siguiente pregunta.
10. Se repite el ciclo.
11. El host puede finalizar la partida.

Reglas:

Solo el host puede iniciar partida.
Solo el host puede generar preguntas.
Solo el host puede generar la siguiente pregunta.
Los demás jugadores esperan y reciben la pregunta activa por polling. 16. Endpoints principales del API Gateway

Base URL:

http://localhost:8080
Health
GET /health
Usuarios
POST /api/users
POST /api/login
Preguntas
POST /api/questions/generate
GET /api/anime/search?q=naruto
Salas
POST /api/rooms
POST /api/rooms/:room_id/join
POST /api/rooms/:room_id/start
POST /api/rooms/:room_id/answer
GET /api/rooms/:room_id/state
GET /api/rooms/:room_id/leaderboard?limit=10
POST /api/rooms/:room_id/end 17. Pruebas backend desde PowerShell

Las pruebas de backend se hacen directamente contra el API Gateway.

En pruebas realizadas, /health respondió correctamente con status: ok y service: api-gateway. También se validó que force_new=false devuelve question: null antes de que exista una pregunta activa, y que luego de generar una pregunta con force_new=true, otra consulta con force_new=false devuelve el mismo question.id.

17.1 Health
Invoke-RestMethod http://localhost:8080/health

Respuesta esperada:

{
"status": "ok",
"service": "api-gateway"
}
17.2 Crear usuario host
$hostUser = Invoke-RestMethod `
  -Method Post `
  -Uri "http://localhost:8080/api/users" `
  -ContentType "application/json" `
  -Body (@{
    username = "host"
    email = "host$(Get-Random)@test.com"
password = "123456"
} | ConvertTo-Json)

$hostUser | ConvertTo-Json -Depth 10
$hostId = $hostUser.user.id
$hostId
17.3 Crear sala
$roomResp = Invoke-RestMethod `  -Method Post`
-Uri "http://localhost:8080/api/rooms" `  -ContentType "application/json"`
-Body (@{
name = "Sala Terminal"
created_by = $hostId
} | ConvertTo-Json)

$roomResp | ConvertTo-Json -Depth 10
$roomId = $roomResp.room.id
$roomId
17.4 Unir host a la sala
Invoke-RestMethod `  -Method Post`
-Uri "http://localhost:8080/api/rooms/$roomId/join" `
  -ContentType "application/json" `
  -Body (@{
    user_id = $hostId
    username = "host"
  } | ConvertTo-Json) | ConvertTo-Json -Depth 10
17.5 Crear jugador 2
$p2 = Invoke-RestMethod `  -Method Post`
-Uri "http://localhost:8080/api/users" `  -ContentType "application/json"`
-Body (@{
username = "player2"
email = "player2$(Get-Random)@test.com"
password = "123456"
} | ConvertTo-Json)

$p2 | ConvertTo-Json -Depth 10
$p2Id = $p2.user.id
$p2Id
17.6 Unir jugador 2
Invoke-RestMethod `  -Method Post`
-Uri "http://localhost:8080/api/rooms/$roomId/join" `
  -ContentType "application/json" `
  -Body (@{
    user_id = $p2Id
    username = "player2"
  } | ConvertTo-Json) | ConvertTo-Json -Depth 10
17.7 Iniciar partida
Invoke-RestMethod `
  -Method Post `
  -Uri "http://localhost:8080/api/rooms/$roomId/start" `  -ContentType "application/json"`
-Body "{}" | ConvertTo-Json -Depth 10
17.8 Consultar pregunta activa antes de generar

Debe devolver:

{
"question": null
}

Comando:

$q0 = Invoke-RestMethod `  -Method Post`
-Uri "http://localhost:8080/api/questions/generate" `  -ContentType "application/json"`
-Body (@{
room_id = $roomId
force_new = $false
} | ConvertTo-Json)

$q0 | ConvertTo-Json -Depth 10
17.9 Host genera pregunta
$q1 = Invoke-RestMethod `  -Method Post`
-Uri "http://localhost:8080/api/questions/generate" `  -ContentType "application/json"`
-Body (@{
room_id = $roomId
force_new = $true
} | ConvertTo-Json)

$q1 | ConvertTo-Json -Depth 10

$questionId = $q1.question.id
$correct = $q1.question.correctOption

$questionId
$correct
17.10 Verificar que otro jugador recibe la misma pregunta
$qCheck = Invoke-RestMethod `  -Method Post`
-Uri "http://localhost:8080/api/questions/generate" `  -ContentType "application/json"`
-Body (@{
room_id = $roomId
force_new = $false
} | ConvertTo-Json)

$qCheck.question.id -eq $questionId

Respuesta esperada:

True
17.11 Host responde
Invoke-RestMethod `  -Method Post`
-Uri "http://localhost:8080/api/rooms/$roomId/answer" `
  -ContentType "application/json" `
  -Body (@{
    user_id = $hostId
    question_id = $questionId
    selected_option = $correct
    correct_option = $correct
  } | ConvertTo-Json) | ConvertTo-Json -Depth 10
17.12 Estado después de una respuesta
Invoke-RestMethod `
  -Uri "http://localhost:8080/api/rooms/$roomId/state?question_id=$questionId" |
ConvertTo-Json -Depth 10

Se espera:

answeredCount = 1
allAnswered = false
17.13 Jugador 2 responde
Invoke-RestMethod `  -Method Post`
-Uri "http://localhost:8080/api/rooms/$roomId/answer" `
  -ContentType "application/json" `
  -Body (@{
    user_id = $p2Id
    question_id = $questionId
    selected_option = "A"
    correct_option = $correct
  } | ConvertTo-Json) | ConvertTo-Json -Depth 10
17.14 Estado después de dos respuestas
Invoke-RestMethod `
  -Uri "http://localhost:8080/api/rooms/$roomId/state?question_id=$questionId" |
ConvertTo-Json -Depth 10

Se espera:

answeredCount = 2
allAnswered = true
17.15 Generar siguiente pregunta
$q2 = Invoke-RestMethod `  -Method Post`
-Uri "http://localhost:8080/api/questions/generate" `  -ContentType "application/json"`
-Body (@{
room_id = $roomId
force_new = $true
} | ConvertTo-Json)

$q2 | ConvertTo-Json -Depth 10
$q2.question.id -ne $questionId

Respuesta esperada:

True
17.16 Consultar leaderboard
Invoke-RestMethod `  -Uri "http://localhost:8080/api/rooms/$roomId/leaderboard?limit=10" |
  ConvertTo-Json -Depth 10
17.17 Finalizar partida
Invoke-RestMethod`
-Method Post `  -Uri "http://localhost:8080/api/rooms/$roomId/end"`
-ContentType "application/json" `
-Body "{}" | ConvertTo-Json -Depth 10 18. Comandos útiles de Docker

Ver contenedores activos:

docker compose ps

Ver logs de todos los servicios:

docker compose logs

Ver logs del gateway:

docker compose logs api-gateway

Ver logs de questions-service:

docker compose logs questions-service

Ver logs de game-room-service:

docker compose logs game-room-service

Ver logs de score-service:

docker compose logs score-service

Detener servicios:

docker compose down

Detener y borrar volúmenes:

docker compose down -v

Advertencia:

docker compose down -v borra datos de PostgreSQL y Redis. 19. Errores comunes
19.1 created_by: invalid type: null

Causa:

$hostId está vacío.

Solución:

$hostId

Si no imprime nada, crea el usuario host de nuevo y asigna:

$hostId = $hostUser.user.id

Este error fue observado durante pruebas al intentar crear una sala antes de inicializar correctamente $hostId.

19.2 room_id must not be empty

Causa:

$roomId está vacío porque falló la creación de la sala.

Solución:

$roomId

Si no imprime nada, crear la sala de nuevo.

19.3 Los jugadores ven preguntas diferentes

Causa probable:

questions-service no está compartiendo la pregunta activa por room_id.

Solución:

docker compose build --no-cache questions-service api-gateway
docker compose up --build

Validación:

$qCheck.question.id -eq $questionId

Debe devolver:

True
19.4 Los jugadores se quedan esperando pregunta

Causa probable:

El host no generó pregunta con force_new=true.

Validar que el host use:

{
"room_id": "ROOM_ID",
"force_new": true
}

Los demás jugadores deben consultar con:

{
"room_id": "ROOM_ID",
"force_new": false
}
19.5 CORS en frontend

Para evitar CORS en local, usar proxy de Vite y dejar:

VITE_API_BASE_URL=

No usar:

VITE_API_BASE_URL=http://localhost:8080

ni:

VITE_API_BASE_URL=http://192.168.x.x:8080 20. Cumplimiento académico

El proyecto cumple con los requisitos principales:

Microservicios en contenedores
api-gateway
users-service
questions-service
game-room-service
score-service
live-events-service

Todos se ejecutan con Docker Compose.

Múltiples bases de datos
PostgreSQL:
users-db
rooms-db

Redis:
score-service
Servicios desarrollados en Rust

Los servicios backend están implementados en Rust.

RPC síncrona, asíncrona y streaming
RPC unaria:
usuarios
salas
preguntas
puntajes

RPC asíncrona:
generación de preguntas
actualización de puntajes
consultas periódicas

Streaming bidireccional:
live-events-service
Clientes
Frontend React/Vite
Pruebas PowerShell contra API Gateway
Clientes Python/Java pueden agregarse para pruebas gRPC/HTTP
Patrones de diseño
API Gateway
Database per Service
Adapter
Circuit Breaker
DTO / Mapper
Service Layer
Event Streaming / Pub-Sub 21. Estado actual del proyecto

Actualmente el sistema permite:

Crear usuarios.
Crear salas.
Unir varios jugadores.
Jugar desde LAN.
Jugar remotamente usando ZeroTier.
Generar preguntas solo desde el host.
Sincronizar preguntas para todos los jugadores.
Registrar respuestas.
Actualizar contador de respuestas.
Actualizar leaderboard.
Generar siguiente pregunta.
Finalizar partida.

El backend fue validado desde terminal con PowerShell: se comprobó health, creación de sala, unión de jugadores, consulta de pregunta activa, generación de pregunta con force_new=true y verificación de que otro cliente recibe el mismo question.id.

22. Comando rápido para demo

Backend:

cd C:\work\Githubdesk\proynube\anime-quiz-arena-ucab
docker compose up --build

Frontend:

cd C:\work\Githubdesk\proynube\anime-quiz-arena-ucab\web
npm run dev -- --host 0.0.0.0

Acceso local:

http://localhost:5173

Acceso por LAN:

http://192.168.1.91:5173

Acceso por ZeroTier:

http://10.103.39.100:5173 23. Resumen final

Anime Quiz Arena es un sistema distribuido multijugador basado en microservicios. El proyecto utiliza Rust, gRPC, Docker Compose, PostgreSQL, Redis y React para construir una trivia anime en tiempo real.

El sistema demuestra:

Arquitectura de microservicios.
Comunicación RPC.
Separación por servicios.
Múltiples bases de datos.
Orquestación local.
API Gateway.
Patrones de diseño.
Juego multijugador funcional.
Pruebas locales, LAN y remotas mediante ZeroTier.
```
