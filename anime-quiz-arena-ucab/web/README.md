# Web Anime Quiz Arena UCAB
Requiere Node.js + npm y backend levantado por Docker Compose.

## Instalar
npm install

## Ejecutar
npm run dev

## Build
npm run build

## Backend requerido
`docker compose up --build users-db rooms-db redis users-service questions-service score-service game-room-service api-gateway`

## Variable
`VITE_API_BASE_URL=http://localhost:8080`

## Flujo demo
Crear usuario -> crear sala -> copiar room_id -> abrir otra ventana -> login -> unirse -> iniciar -> responder -> ver leaderboard.

## Problemas comunes
CORS, gateway apagado, puerto 8080/5173 ocupado.
