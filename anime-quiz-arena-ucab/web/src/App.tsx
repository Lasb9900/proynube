import { useCallback, useEffect, useMemo, useState } from "react";
import confetti from "canvas-confetti";
import { toast } from "sonner";

import * as api from "./api";
import { Leaderboard } from "./components/Leaderboard";
import { Lobby } from "./components/Lobby";
import { LoginPanel } from "./components/LoginPanel";
import { QuestionCard } from "./components/QuestionCard";

import type { Question, Room, ScoreEntry, User } from "./types";

import "./styles.css";

function getStoredUser(): User | null {
  try {
    return JSON.parse(localStorage.getItem("user") || "null");
  } catch {
    localStorage.removeItem("user");
    return null;
  }
}

export default function App() {
  const [user, setUser] = useState<User | null>(getStoredUser());
  const [room, setRoom] = useState<Room | null>(null);
  const [question, setQuestion] = useState<Question | null>(null);
  const [leaderboard, setLeaderboard] = useState<ScoreEntry[]>([]);
  const [loadingAction, setLoadingAction] = useState<string | null>(null);

  const hasRoom = Boolean(room?.id);

  const refreshLeaderboard = useCallback(() => {
    if (!room?.id) return;

    api
      .getLeaderboard(room.id, 10)
      .then((response) => setLeaderboard(response.entries))
      .catch(() => {});
  }, [room?.id]);

  useEffect(() => {
    if (!room?.id) return;

    refreshLeaderboard();

    const interval = setInterval(refreshLeaderboard, 2000);
    return () => clearInterval(interval);
  }, [room?.id, refreshLeaderboard]);

  const handleUser = (newUser: User) => {
    setUser(newUser);
    localStorage.setItem("user", JSON.stringify(newUser));
    toast.success("Sesión iniciada");
  };

  const handleLogout = () => {
    localStorage.removeItem("user");
    setUser(null);
    setRoom(null);
    setQuestion(null);
    setLeaderboard([]);
    toast.success("Sesión cerrada");
  };

  const handleStartGame = async () => {
    if (!room?.id) return;

    try {
      setLoadingAction("start");
      const response = await api.startGame(room.id);
      setRoom(response.room);
      toast.success("Partida iniciada");
    } catch {
      toast.error("No se pudo iniciar la partida");
    } finally {
      setLoadingAction(null);
    }
  };

  const handleGenerateQuestion = async () => {
    if (!room?.id) return;

    try {
      setLoadingAction("question");
      const response = await api.generateQuestion(room.id);
      setQuestion(response.question);
      toast.success("Pregunta generada");
    } catch {
      toast.error("No se pudo generar la pregunta");
    } finally {
      setLoadingAction(null);
    }
  };

  const handleEndGame = async () => {
    if (!room?.id) return;

    try {
      setLoadingAction("end");
      const response = await api.endGame(room.id);
      setRoom(response.room);
      toast.success("Partida finalizada");
    } catch {
      toast.error("No se pudo finalizar la partida");
    } finally {
      setLoadingAction(null);
    }
  };

  const handleAnswer = async (option: string) => {
    if (!room?.id || !user?.id || !question?.id) {
      toast.error("Faltan datos de sala, usuario o pregunta");
      return;
    }

    if (!question.correctOption) {
      toast.error("La pregunta no tiene respuesta correcta cargada");
      return;
    }

    try {
      setLoadingAction("answer");

      // Asegura que el jugador esté unido a la sala antes de responder.
      // JoinRoom es idempotente en el backend, así que no debería romper si ya estaba unido.
      try {
        await api.joinRoom(room.id, user.id);
      } catch (error) {
        console.warn("join before answer failed", error);
      }

      // Si la sala sigue en WAITING, intenta iniciarla antes de responder.
      let activeRoom = room;
      if (room.status === "WAITING") {
        const started = await api.startGame(room.id);
        if (started.room) {
          activeRoom = started.room;
          setRoom(started.room);
        }
      }

      if (activeRoom.status === "FINISHED") {
        toast.error("La partida ya finalizó");
        return;
      }

      const response = await api.submitAnswer(
        room.id,
        user.id,
        question.id,
        option,
        question.correctOption,
      );

      if (response.correct) {
        confetti({
          particleCount: 120,
          spread: 75,
          origin: { y: 0.65 },
        });

        toast.success(`+${response.pointsAwarded ?? 100} puntos`);
      } else {
        toast.error("Respuesta incorrecta");
      }

      await refreshLeaderboard();
    } catch (error) {
      console.error("submit answer failed", error);
      toast.error(
        error instanceof Error
          ? error.message
          : "No se pudo enviar la respuesta",
      );
    } finally {
      setLoadingAction(null);
    }
  };

  const roomLabel = useMemo(() => {
    if (!room?.id) return "Sin sala activa";
    return room.id;
  }, [room?.id]);

  return (
    <main className="arena-shell">
      <section className="arena-hero">
        <div>
          <span className="arena-eyebrow">UCAB Cloud Project</span>
          <h1>Anime Quiz Arena</h1>
          <p>
            Trivia anime en tiempo real usando microservicios, salas, preguntas
            y ranking en vivo.
          </p>
        </div>

        {user && (
          <div className="user-pill">
            <span>Jugador</span>
            <strong>{user.email || user.id || "Lasb"}</strong>
            <button
              type="button"
              className="ghost-button"
              onClick={handleLogout}
            >
              Salir
            </button>
          </div>
        )}
      </section>

      {!user ? (
        <section className="arena-card auth-card">
          <LoginPanel onUser={handleUser} />
        </section>
      ) : (
        <>
          <section className="arena-card lobby-card">
            <div className="section-header">
              <div>
                <span className="arena-eyebrow">Lobby</span>
                <h2>Sala de juego</h2>
              </div>

              <div className="room-badge">
                <span>Room</span>
                <strong>{roomLabel}</strong>
              </div>
            </div>

            <Lobby user={user} onRoom={setRoom} room={room} />
          </section>

          {hasRoom && (
            <section className="action-bar">
              <button
                type="button"
                className="success-button"
                disabled={loadingAction === "start"}
                onClick={handleStartGame}
              >
                {loadingAction === "start" ? "Iniciando..." : "Iniciar partida"}
              </button>

              <button
                type="button"
                className="primary-button"
                disabled={loadingAction === "question"}
                onClick={handleGenerateQuestion}
              >
                {loadingAction === "question"
                  ? "Generando..."
                  : "Generar pregunta"}
              </button>

              <button
                type="button"
                className="danger-button"
                disabled={loadingAction === "end"}
                onClick={handleEndGame}
              >
                {loadingAction === "end"
                  ? "Finalizando..."
                  : "Finalizar partida"}
              </button>
            </section>
          )}

          <section className="arena-grid">
            <div className="arena-card question-section">
              {question ? (
                <QuestionCard question={question} onAnswer={handleAnswer} />
              ) : (
                <div className="empty-state">
                  <span>🎌</span>
                  <h2>Sin pregunta activa</h2>
                  <p>Genera una pregunta para comenzar la ronda.</p>
                </div>
              )}
            </div>

            <div className="arena-card leaderboard-section">
              <div className="section-header">
                <div>
                  <span className="arena-eyebrow">Ranking</span>
                  <h2>Leaderboard</h2>
                </div>

                <button
                  type="button"
                  className="ghost-button"
                  onClick={refreshLeaderboard}
                >
                  Refrescar
                </button>
              </div>

              <Leaderboard
                entries={leaderboard}
                onRefresh={refreshLeaderboard}
              />
            </div>
          </section>
        </>
      )}
    </main>
  );
}
