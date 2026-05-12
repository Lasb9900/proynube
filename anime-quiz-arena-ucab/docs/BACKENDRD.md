# README Backend - Anime Quiz Arena

Este documento explica cómo levantar y probar únicamente el backend del proyecto **Anime Quiz Arena**.

El backend está compuesto por un API Gateway y varios microservicios en Rust comunicados por gRPC interno. Las pruebas externas se hacen por HTTP usando el API Gateway en el puerto `8080`.

---

## 1. Requisitos previos

Antes de iniciar, asegúrate de tener instalado:

- Docker Desktop
- Docker Compose
- PowerShell
- Git

Opcional para pruebas remotas:

- ZeroTier

---

## 2. Arquitectura backend

El backend está compuesto por:

```txt
api-gateway
users-service
questions-service
game-room-service
score-service
live-events-service
users-db
rooms-db
redis
```
