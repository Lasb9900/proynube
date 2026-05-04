import { useCallback, useEffect, useMemo, useState } from "react";
import confetti from "canvas-confetti";
import { toast } from "sonner";

import * as api from "./api";
import { Leaderboard } from "./components/Leaderboard";
import { Lobby } from "./components/Lobby";
import { LoginPanel } from "./components/LoginPanel";
import { PlayersPanel } from "./components/PlayersPanel";
import { QuestionCard } from "./components/QuestionCard";

import type {
  Question,
  Room,
  RoomPlayer,
  RoomState,
  ScoreEntry,
  User,
} from "./types";

import "./styles.css";

function getStoredUser(): User | null {
  try {
    return JSON.parse(localStorage.getItem("user") || "null");
  } catch {
    localStorage.removeItem("user");
    return null;
  }
}

function isAlreadyAnsweredError(error: unknown) {
  return (
    error instanceof Error &&
    error.message.toLowerCase().includes("already answered")
  );
}

export default function App() {
  const [user, setUser] = useState<User | null>(getStoredUser());
  const [room, setRoom] = useState<Room | null>(null);
  const [roomState, setRoomState] = useState<RoomState | null>(null);
  const [question, setQuestion] = useState<Question | null>(null);
  const [leaderboard, setLeaderboard] = useState<ScoreEntry[]>([]);
  const [loadingAction, setLoadingAction] = useState<string | null>(null);
  const [answeredQuestionIds, setAnsweredQuestionIds] = useState<Set<string>>(
    new Set(),
  );
  const [currentUserAnswered, setCurrentUserAnswered] = useState(false);
  const [selectedOption, setSelectedOption] = useState<string | null>(null);

  const hasRoom = Boolean(room?.id);

  const refreshLeaderboard = useCallback(async () => {
    if (!room?.id) return;

    try {
      const response = await api.getLeaderboard(room.id, 10);
      setLeaderboard(response.entries);
    } catch {
      // Polling silencioso para no molestar durante demo.
    }
  }, [room?.id]);

  const refreshRoomState = useCallback(
    async (questionIdOverride?: string) => {
      if (!room?.id) return null;

      try {
        const response = await api.getRoomState(
          room.id,
          questionIdOverride ?? question?.id,
        );

        setRoomState(response);

        if (response.room) {
          setRoom(response.room);
        }

        const currentPlayer = response.players.find(
          (player: { userId: string; answeredCurrentQuestion: boolean }) =>
            player.userId === user?.id,
        );

        setCurrentUserAnswered(Boolean(currentPlayer?.answeredCurrentQuestion));

        return response;
      } catch {
        return null;
      }
    },
    [question?.id, room?.id, user?.id],
  );

  useEffect(() => {
    if (!room?.id) return;

    refreshLeaderboard();

    const interval = setInterval(refreshLeaderboard, 2000);
    return () => clearInterval(interval);
  }, [room?.id, refreshLeaderboard]);

  useEffect(() => {
    if (!room?.id) return;

    refreshRoomState();

    const interval = setInterval(() => {
      refreshRoomState();
    }, 2000);

    return () => clearInterval(interval);
  }, [room?.id, question?.id, refreshRoomState]);

  const handleUser = (newUser: User) => {
    setUser(newUser);
    localStorage.setItem("user", JSON.stringify(newUser));
    toast.success("Sesión iniciada");
  };

  const handleLogout = () => {
    localStorage.removeItem("user");

    setUser(null);
    setRoom(null);
    setRoomState(null);
    setQuestion(null);
    setLeaderboard([]);
    setAnsweredQuestionIds(new Set());
    setCurrentUserAnswered(false);
    setSelectedOption(null);

    toast.success("Sesión cerrada");
  };

  const handleRoomChange = (nextRoom: Room | null) => {
    setRoom(nextRoom);
    setRoomState(null);
    setQuestion(null);
    setLeaderboard([]);
    setAnsweredQuestionIds(new Set());
    setCurrentUserAnswered(false);
    setSelectedOption(null);
  };

  const handleStartGame = async () => {
    if (!room?.id) return;

    try {
      setLoadingAction("start");

      const response = await api.startGame(room.id);
      setRoom(response.room);

      await refreshRoomState();

      toast.success("Partida iniciada");
    } catch (error) {
      toast.error(
        error instanceof Error
          ? error.message
          : "No se pudo iniciar la partida",
      );
    } finally {
      setLoadingAction(null);
    }
  };

  const handleGenerateQuestion = async () => {
    if (!room?.id) return;

    const canGenerate =
      !question || roomState?.allAnswered || currentUserAnswered;

    if (!canGenerate) {
      toast.error("Aún faltan jugadores por responder");
      return;
    }

    try {
      setLoadingAction("question");

      const response = await api.generateQuestion(room.id);

      setQuestion(response.question);
      setAnsweredQuestionIds(new Set());
      setCurrentUserAnswered(false);
      setSelectedOption(null);

      if (response.question?.id) {
        await refreshRoomState(response.question.id);
      }

      toast.success(
        question ? "Siguiente pregunta lista" : "Pregunta generada",
      );
    } catch (error) {
      toast.error(
        error instanceof Error
          ? error.message
          : "No se pudo generar la pregunta",
      );
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

      await refreshRoomState();

      toast.success("Partida finalizada");
    } catch (error) {
      toast.error(
        error instanceof Error
          ? error.message
          : "No se pudo finalizar la partida",
      );
    } finally {
      setLoadingAction(null);
    }
  };

  const handleAnswer = async (option: string) => {
    if (!room?.id || !user?.id || !question?.id) return;

    if (answeredQuestionIds.has(question.id) || currentUserAnswered) {
      toast.error("Ya respondiste esta pregunta");
      return;
    }

    try {
      setLoadingAction("answer");
      setSelectedOption(option);

      const response = await api.submitAnswer(
        room.id,
        user.id,
        question.id,
        option,
        question.correctOption,
      );

      setAnsweredQuestionIds((prev) => new Set(prev).add(question.id));
      setCurrentUserAnswered(true);

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
      await refreshRoomState(question.id);
    } catch (error) {
      if (isAlreadyAnsweredError(error)) {
        setAnsweredQuestionIds((prev) => new Set(prev).add(question.id));
        setCurrentUserAnswered(true);

        await refreshRoomState(question.id);

        toast.error("Ya respondiste esta pregunta");
        return;
      }

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

  const playerNames = useMemo(() => {
    const map: Record<string, string> = {};

    roomState?.players.forEach((player) => {
      map[player.userId] = player.username || player.userId.slice(0, 8);
    });

    if (user?.id) {
      map[user.id] = user.username;
    }

    return map;
  }, [roomState, user]);

  const totalPlayers = roomState?.totalPlayers ?? 1;
  const answeredCount =
    roomState?.answeredCount ?? (currentUserAnswered ? 1 : 0);
  const allAnswered = roomState?.allAnswered ?? currentUserAnswered;

  const canGenerateQuestion = Boolean(room?.id) && (!question || allAnswered);

  const questionButtonLabel = !question
    ? "Generar pregunta"
    : allAnswered
      ? "Siguiente pregunta"
      : "Esperando jugadores...";

  const roundMessage = !question
    ? "Genera una pregunta para comenzar la ronda."
    : allAnswered
      ? "Todos respondieron. Puedes generar la siguiente pregunta."
      : `Esperando respuestas... ${answeredCount}/${totalPlayers} jugadores respondieron.`;

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
            <strong>{user.username || user.email || user.id}</strong>
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

            <Lobby user={user} onRoom={handleRoomChange} room={room} />
          </section>

          {hasRoom && (
            <>
              <section className="action-bar">
                <button
                  type="button"
                  className="success-button"
                  disabled={!!loadingAction || room?.status === "STARTED"}
                  onClick={handleStartGame}
                >
                  {loadingAction === "start"
                    ? "Iniciando..."
                    : room?.status === "STARTED"
                      ? "Partida iniciada"
                      : "Iniciar partida"}
                </button>

                <button
                  type="button"
                  className="primary-button"
                  disabled={!!loadingAction || !canGenerateQuestion}
                  onClick={handleGenerateQuestion}
                >
                  {loadingAction === "question"
                    ? "Generando..."
                    : questionButtonLabel}
                </button>

                <button
                  type="button"
                  className="danger-button"
                  disabled={!!loadingAction || room?.status === "FINISHED"}
                  onClick={handleEndGame}
                >
                  {loadingAction === "end"
                    ? "Finalizando..."
                    : room?.status === "FINISHED"
                      ? "Partida finalizada"
                      : "Finalizar partida"}
                </button>
              </section>

              <section className="round-status">
                <strong>
                  {answeredCount}/{totalPlayers} jugadores respondieron
                </strong>
                <p>{roundMessage}</p>
              </section>
            </>
          )}

          <section className="arena-grid">
            <div className="arena-card question-section">
              {room && (
                <PlayersPanel roomState={roomState} currentUserId={user?.id} />
              )}

              {question ? (
                <QuestionCard
                  question={question}
                  onAnswer={handleAnswer}
                  disabled={loadingAction === "answer" || currentUserAnswered}
                  answered={currentUserAnswered}
                  selectedOption={selectedOption}
                  correctOption={question.correctOption}
                />
              ) : (
                <div className="empty-state">
                  <span>🎌</span>
                  <h2>Sin pregunta activa</h2>
                  <p>Genera una pregunta para comenzar la ronda.</p>
                </div>
              )}
            </div>

            <div className="arena-card leaderboard-section">
              <Leaderboard
                entries={leaderboard}
                onRefresh={refreshLeaderboard}
                playerNames={playerNames}
              />
            </div>
          </section>
        </>
      )}
    </main>
  );
}
