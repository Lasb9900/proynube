# users-service

Servicio gRPC para gestión básica de usuarios del juego (registro, consulta y login simple) usando PostgreSQL.

## Puerto
- gRPC: `50051`

## Variable de entorno
- `DATABASE_URL=postgres://postgres:postgres@users-db:5432/users_db`

## Tabla PostgreSQL
Tabla `users` (se crea automáticamente al iniciar si no existe):
- `id UUID PRIMARY KEY`
- `username VARCHAR NOT NULL`
- `email VARCHAR NOT NULL UNIQUE`
- `password VARCHAR NOT NULL`
- `created_at TIMESTAMP NOT NULL`

> Nota académica: el password se compara en texto plano para simplificar la fase. En producción debe usarse hashing seguro (Argon2/Bcrypt/Scrypt + salt).

## Métodos gRPC

### `CreateUser`
- Valida `username`, `email` y `password`.
- Genera UUID.
- Inserta en PostgreSQL.
- Responde `User` sin password.
- Si `email` existe, responde `already_exists`.

### `GetUser`
- Busca por `id`.
- Si no existe, responde `not_found`.
- Responde `User` sin password.

### `LoginBasic`
- Busca por `email`.
- Compara `password` simple.
- Si coincide: `success = true` y `user`.
- Si no coincide: `unauthenticated`.

## Ejecutar con Docker
Desde la raíz del proyecto (`anime-quiz-arena-ucab`):

```bash
docker compose up --build users-db users-service
```

Logs esperados:
- `Starting users-service on 0.0.0.0:50051`
- `Connected to PostgreSQL`
- `Users table ready`

## Probar con grpcurl
Desde la raíz del repo:

### 1) CreateUser
```bash
grpcurl -plaintext \
  -import-path proto \
  -proto users.proto \
  -d '{"username":"lasb","email":"lasb@ucab.edu.ve","password":"123456"}' \
  localhost:50051 animequiz.users.v1.UsersService/CreateUser
```

Guarda el `id` devuelto.

### 2) GetUser
```bash
grpcurl -plaintext \
  -import-path proto \
  -proto users.proto \
  -d '{"id":"REEMPLAZAR_UUID"}' \
  localhost:50051 animequiz.users.v1.UsersService/GetUser
```

### 3) LoginBasic
```bash
grpcurl -plaintext \
  -import-path proto \
  -proto users.proto \
  -d '{"email":"lasb@ucab.edu.ve","password":"123456"}' \
  localhost:50051 animequiz.users.v1.UsersService/LoginBasic
```

## Patrones aplicados
- **Microservices**
- **Database per Service**
- **SQL persistence**
