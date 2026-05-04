import { useCallback, useEffect, useMemo, useState } from "react";
import confetti from "canvas-confetti";
import { toast } from "sonner";

import * as api from "./api";
import { Leaderboard } from "./components/Leaderboard";
import { Lobby } from "./components/Lobby";
import { LoginPanel } from "./components/LoginPanel";
import { PlayersPanel } from "./components/PlayersPanel";
import { QuestionCard } from "./components/QuestionCard";

import type { Question, Room, RoomState, ScoreEntry, User } from "./types";

import "./styles.css";

function getStoredUser(): User | null {
  try {
    return JSON.parse(localStorage.getItem("user") || "null");
  } catch {
    return null;
  }
}

export default function App() {
  const [user, setUser] = useState<User | null>(getStoredUser());
  const [room, setRoom] = useState<Room | null>(null);
  const [question, setQuestion] = useState<Question | null>(null);
  const [leaderboard, setLeaderboard] = useState<ScoreEntry[]>([]);
  const [roomState, setRoomState] = useState<RoomState | null>(null);
  const [answeredQuestionIds, setAnsweredQuestionIds] = useState<Set<string>>(new Set());
  const [currentUserAnswered, setCurrentUserAnswered] = useState(false);
  const [selectedOption, setSelectedOption] = useState<string>();
  const [loadingAction, setLoadingAction] = useState<string | null>(null);

  const hasRoom = Boolean(room?.id);
  const totalPlayers = roomState?.totalPlayers ?? 0;
  const answeredCount = roomState?.answeredCount ?? 0;
  const allAnswered = roomState?.allAnswered ?? false;
  const hasActiveQuestion = Boolean(question?.id);

  const canGenerateQuestion =
    hasRoom && !loadingAction && (!hasActiveQuestion || allAnswered);

  const questionButtonLabel = !hasActiveQuestion
    ? "Generar pregunta"
    : allAnswered
      ? "Siguiente pregunta"
      : "Esperando jugadores...";

  const roundMessage = !hasActiveQuestion
    ? "Genera una pregunta para iniciar la ronda."
    : allAnswered
      ? "Todos respondieron. Puedes generar la siguiente pregunta."
      : "Esperando respuestas de los demas jugadores...";

  const refreshLeaderboard = useCallback(async () => {
    if (!room?.id) return;

    try {
      const response = await api.getLeaderboard(room.id, 10);
      setLeaderboard(response.entries);
    } catch {
      // noop
    }
  }, [room?.id]);

  const refreshRoomState = useCallback(async () => {
    if (!room?.id) return;

    const response = await api.getRoomState(room.id, question?.id);
    setRoomState(response);

    if (response.room) {
      setRoom(response.room);
    }

    const currentPlayer = response.players.find((p: any) => p.userId === user?.id);
    setCurrentUserAnswered(currentPlayer?.answeredCurrentQuestion ?? false);
  }, [room?.id, question?.id, user?.id]);

  useEffect(() => {
    if (!room?.id) return;

    refreshLeaderboard();
    refreshRoomState();

    const interval = setInterval(() => {
      refreshLeaderboard();
      refreshRoomState();
    }, 2000);

    return () => clearInterval(interval);
  }, [room?.id, refreshLeaderboard, refreshRoomState]);

  const handleUser = (newUser: User) => {
    setUser(newUser);
    localStorage.setItem("user", JSON.stringify(newUser));
    toast.success("Sesion iniciada");
  };

  const handleGenerateQuestion = async () => {
    if (!room?.id) return;
    if (!canGenerateQuestion) {
      toast.error("Aun faltan jugadores por responder");
      return;
    }

    try {
      setLoadingAction("question");
      const response = await api.generateQuestion(room.id);

      setQuestion(response.question);
      setAnsweredQuestionIds(new Set());
      setCurrentUserAnswered(false);
      setSelectedOption(undefined);

      await refreshRoomState();
      toast.success("Pregunta generada");
    } catch {
      toast.error("No se pudo generar la pregunta");
    } finally {
      setLoadingAction(null);
    }
  };

  const handleAnswer = async (option: string) => {
    if (!room?.id || !user?.id || !question?.id) return;

    if (currentUserAnswered || answeredQuestionIds.has(question.id)) {
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

      if (response.correct) {
        confetti({ particleCount: 100, spread: 70, origin: { y: 0.7 } });
        toast.success(`+${response.pointsAwarded ?? 100} puntos`);
      } else {
        toast.error("Respuesta incorrecta");
      }

      setAnsweredQuestionIds((prev) => new Set(prev).add(question.id));
      setCurrentUserAnswered(true);

      await refreshLeaderboard();
      await refreshRoomState();
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "No se pudo enviar la respuesta";

      if (message.includes("already answered") || message.includes("412")) {
        toast.error("Ya respondiste esta pregunta");
        setAnsweredQuestionIds((prev) => new Set(prev).add(question.id));
        setCurrentUserAnswered(true);
        await refreshRoomState();
        await refreshLeaderboard();
      } else {
        toast.error(message);
      }
    } finally {
      setLoadingAction(null);
    }
  };

  const playerNames = useMemo(() => {
    const map: Record<string, string> = {};

    roomState?.players.forEach((p) => {
      map[p.userId] = p.username || p.userId.slice(0, 8);
    });

    if (user?.id) map[user.id] = user.username;
    return map;
  }, [roomState, user]);

  return (
    <main className="arena-shell">
      {!user ? (
        <section className="arena-card auth-card">
          <LoginPanel onUser={handleUser} />
        </section>
      ) : (
        <>
          <section className="arena-card lobby-card">
            <Lobby
              user={user}
              room={room}
              onRoom={setRoom}
              onRefreshRoomState={refreshRoomState}
            />
          </section>

          {hasRoom && (
            <section className="arena-card round-banner">
              <strong>
                {answeredCount}/{totalPlayers} jugadores respondieron
              </strong>
              <p className="muted">{roundMessage}</p>
            </section>
          )}

          {hasRoom && (
            <section className="action-bar">
              <button
                type="button"
                className="primary-button"
                disabled={!canGenerateQuestion}
                onClick={handleGenerateQuestion}
              >
                {loadingAction === "question" ? "Generando..." : questionButtonLabel}
              </button>
            </section>
          )}

          <section className="arena-grid">
            <div className="arena-card question-section">
              {question ? (
                <QuestionCard
                  question={question}
                  onAnswer={handleAnswer}
                  disabled={loadingAction === "answer" || currentUserAnswered}
                  selectedOption={selectedOption}
                  correctOption={question.correctOption}
                  answered={currentUserAnswered}
                />
              ) : (
                <div className="empty-state">
                  <h2>Sin pregunta activa</h2>
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

          {hasRoom && (
            <section className="arena-card">
              <PlayersPanel roomState={roomState} currentUserId={user?.id} />
            </section>
          )}
        </>
      )}
    </main>
  );
}
