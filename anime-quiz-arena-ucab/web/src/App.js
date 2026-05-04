import { jsx as _jsx, jsxs as _jsxs, Fragment as _Fragment } from "react/jsx-runtime";
import { useCallback, useEffect, useMemo, useState } from "react";
import confetti from "canvas-confetti";
import { toast } from "sonner";
import * as api from "./api";
import { Leaderboard } from "./components/Leaderboard";
import { Lobby } from "./components/Lobby";
import { LoginPanel } from "./components/LoginPanel";
import { PlayersPanel } from "./components/PlayersPanel";
import { QuestionCard } from "./components/QuestionCard";
import "./styles.css";
function getStoredUser() {
    try {
        return JSON.parse(localStorage.getItem("user") || "null");
    }
    catch {
        localStorage.removeItem("user");
        return null;
    }
}
function isAlreadyAnsweredError(error) {
    return (error instanceof Error &&
        error.message.toLowerCase().includes("already answered"));
}
export default function App() {
    const [user, setUser] = useState(getStoredUser());
    const [room, setRoom] = useState(null);
    const [roomState, setRoomState] = useState(null);
    const [question, setQuestion] = useState(null);
    const [leaderboard, setLeaderboard] = useState([]);
    const [loadingAction, setLoadingAction] = useState(null);
    const [answeredQuestionIds, setAnsweredQuestionIds] = useState(new Set());
    const [currentUserAnswered, setCurrentUserAnswered] = useState(false);
    const [selectedOption, setSelectedOption] = useState(null);
    const hasRoom = Boolean(room?.id);
    const refreshLeaderboard = useCallback(async () => {
        if (!room?.id)
            return;
        try {
            const response = await api.getLeaderboard(room.id, 10);
            setLeaderboard(response.entries);
        }
        catch {
            // Polling silencioso para no molestar durante demo.
        }
    }, [room?.id]);
    const refreshRoomState = useCallback(async (questionIdOverride) => {
        if (!room?.id)
            return null;
        try {
            const response = await api.getRoomState(room.id, questionIdOverride ?? question?.id);
            setRoomState(response);
            if (response.room) {
                setRoom(response.room);
            }
            const currentPlayer = response.players.find((player) => player.userId === user?.id);
            setCurrentUserAnswered(Boolean(currentPlayer?.answeredCurrentQuestion));
            return response;
        }
        catch {
            return null;
        }
    }, [question?.id, room?.id, user?.id]);
    useEffect(() => {
        if (!room?.id)
            return;
        refreshLeaderboard();
        const interval = setInterval(refreshLeaderboard, 2000);
        return () => clearInterval(interval);
    }, [room?.id, refreshLeaderboard]);
    useEffect(() => {
        if (!room?.id)
            return;
        refreshRoomState();
        const interval = setInterval(() => {
            refreshRoomState();
        }, 2000);
        return () => clearInterval(interval);
    }, [room?.id, question?.id, refreshRoomState]);
    const handleUser = (newUser) => {
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
    const handleRoomChange = (nextRoom) => {
        setRoom(nextRoom);
        setRoomState(null);
        setQuestion(null);
        setLeaderboard([]);
        setAnsweredQuestionIds(new Set());
        setCurrentUserAnswered(false);
        setSelectedOption(null);
    };
    const handleStartGame = async () => {
        if (!room?.id)
            return;
        try {
            setLoadingAction("start");
            const response = await api.startGame(room.id);
            setRoom(response.room);
            await refreshRoomState();
            toast.success("Partida iniciada");
        }
        catch (error) {
            toast.error(error instanceof Error
                ? error.message
                : "No se pudo iniciar la partida");
        }
        finally {
            setLoadingAction(null);
        }
    };
    const handleGenerateQuestion = async () => {
        if (!room?.id)
            return;
        const canGenerate = !question || roomState?.allAnswered || currentUserAnswered;
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
            toast.success(question ? "Siguiente pregunta lista" : "Pregunta generada");
        }
        catch (error) {
            toast.error(error instanceof Error
                ? error.message
                : "No se pudo generar la pregunta");
        }
        finally {
            setLoadingAction(null);
        }
    };
    const handleEndGame = async () => {
        if (!room?.id)
            return;
        try {
            setLoadingAction("end");
            const response = await api.endGame(room.id);
            setRoom(response.room);
            await refreshRoomState();
            toast.success("Partida finalizada");
        }
        catch (error) {
            toast.error(error instanceof Error
                ? error.message
                : "No se pudo finalizar la partida");
        }
        finally {
            setLoadingAction(null);
        }
    };
    const handleAnswer = async (option) => {
        if (!room?.id || !user?.id || !question?.id)
            return;
        if (answeredQuestionIds.has(question.id) || currentUserAnswered) {
            toast.error("Ya respondiste esta pregunta");
            return;
        }
        try {
            setLoadingAction("answer");
            setSelectedOption(option);
            const response = await api.submitAnswer(room.id, user.id, question.id, option, question.correctOption);
            setAnsweredQuestionIds((prev) => new Set(prev).add(question.id));
            setCurrentUserAnswered(true);
            if (response.correct) {
                confetti({
                    particleCount: 120,
                    spread: 75,
                    origin: { y: 0.65 },
                });
                toast.success(`+${response.pointsAwarded ?? 100} puntos`);
            }
            else {
                toast.error("Respuesta incorrecta");
            }
            await refreshLeaderboard();
            await refreshRoomState(question.id);
        }
        catch (error) {
            if (isAlreadyAnsweredError(error)) {
                setAnsweredQuestionIds((prev) => new Set(prev).add(question.id));
                setCurrentUserAnswered(true);
                await refreshRoomState(question.id);
                toast.error("Ya respondiste esta pregunta");
                return;
            }
            toast.error(error instanceof Error
                ? error.message
                : "No se pudo enviar la respuesta");
        }
        finally {
            setLoadingAction(null);
        }
    };
    const roomLabel = useMemo(() => {
        if (!room?.id)
            return "Sin sala activa";
        return room.id;
    }, [room?.id]);
    const playerNames = useMemo(() => {
        const map = {};
        roomState?.players.forEach((player) => {
            map[player.userId] = player.username || player.userId.slice(0, 8);
        });
        if (user?.id) {
            map[user.id] = user.username;
        }
        return map;
    }, [roomState, user]);
    const totalPlayers = roomState?.totalPlayers ?? 1;
    const answeredCount = roomState?.answeredCount ?? (currentUserAnswered ? 1 : 0);
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
    return (_jsxs("main", { className: "arena-shell", children: [_jsxs("section", { className: "arena-hero", children: [_jsxs("div", { children: [_jsx("span", { className: "arena-eyebrow", children: "UCAB Cloud Project" }), _jsx("h1", { children: "Anime Quiz Arena" }), _jsx("p", { children: "Trivia anime en tiempo real usando microservicios, salas, preguntas y ranking en vivo." })] }), user && (_jsxs("div", { className: "user-pill", children: [_jsx("span", { children: "Jugador" }), _jsx("strong", { children: user.username || user.email || user.id }), _jsx("button", { type: "button", className: "ghost-button", onClick: handleLogout, children: "Salir" })] }))] }), !user ? (_jsx("section", { className: "arena-card auth-card", children: _jsx(LoginPanel, { onUser: handleUser }) })) : (_jsxs(_Fragment, { children: [_jsxs("section", { className: "arena-card lobby-card", children: [_jsxs("div", { className: "section-header", children: [_jsxs("div", { children: [_jsx("span", { className: "arena-eyebrow", children: "Lobby" }), _jsx("h2", { children: "Sala de juego" })] }), _jsxs("div", { className: "room-badge", children: [_jsx("span", { children: "Room" }), _jsx("strong", { children: roomLabel })] })] }), _jsx(Lobby, { user: user, onRoom: handleRoomChange, room: room })] }), hasRoom && (_jsxs(_Fragment, { children: [_jsxs("section", { className: "action-bar", children: [_jsx("button", { type: "button", className: "success-button", disabled: !!loadingAction || room?.status === "STARTED", onClick: handleStartGame, children: loadingAction === "start"
                                            ? "Iniciando..."
                                            : room?.status === "STARTED"
                                                ? "Partida iniciada"
                                                : "Iniciar partida" }), _jsx("button", { type: "button", className: "primary-button", disabled: !!loadingAction || !canGenerateQuestion, onClick: handleGenerateQuestion, children: loadingAction === "question"
                                            ? "Generando..."
                                            : questionButtonLabel }), _jsx("button", { type: "button", className: "danger-button", disabled: !!loadingAction || room?.status === "FINISHED", onClick: handleEndGame, children: loadingAction === "end"
                                            ? "Finalizando..."
                                            : room?.status === "FINISHED"
                                                ? "Partida finalizada"
                                                : "Finalizar partida" })] }), _jsxs("section", { className: "round-status", children: [_jsxs("strong", { children: [answeredCount, "/", totalPlayers, " jugadores respondieron"] }), _jsx("p", { children: roundMessage })] })] })), _jsxs("section", { className: "arena-grid", children: [_jsxs("div", { className: "arena-card question-section", children: [room && (_jsx(PlayersPanel, { roomState: roomState, currentUserId: user?.id })), question ? (_jsx(QuestionCard, { question: question, onAnswer: handleAnswer, disabled: loadingAction === "answer" || currentUserAnswered, answered: currentUserAnswered, selectedOption: selectedOption, correctOption: question.correctOption })) : (_jsxs("div", { className: "empty-state", children: [_jsx("span", { children: "\uD83C\uDF8C" }), _jsx("h2", { children: "Sin pregunta activa" }), _jsx("p", { children: "Genera una pregunta para comenzar la ronda." })] }))] }), _jsx("div", { className: "arena-card leaderboard-section", children: _jsx(Leaderboard, { entries: leaderboard, onRefresh: refreshLeaderboard, playerNames: playerNames }) })] })] }))] }));
}
