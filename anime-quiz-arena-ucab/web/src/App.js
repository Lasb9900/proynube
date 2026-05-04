import { jsx as _jsx, jsxs as _jsxs, Fragment as _Fragment } from "react/jsx-runtime";
import { useCallback, useEffect, useMemo, useState } from "react";
import confetti from "canvas-confetti";
import { toast } from "sonner";
import * as api from "./api";
import { Leaderboard } from "./components/Leaderboard";
import { Lobby } from "./components/Lobby";
import { LoginPanel } from "./components/LoginPanel";
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
export default function App() {
    const [user, setUser] = useState(getStoredUser());
    const [room, setRoom] = useState(null);
    const [question, setQuestion] = useState(null);
    const [leaderboard, setLeaderboard] = useState([]);
    const [loadingAction, setLoadingAction] = useState(null);
    const hasRoom = Boolean(room?.id);
    const refreshLeaderboard = useCallback(() => {
        if (!room?.id)
            return;
        api
            .getLeaderboard(room.id, 10)
            .then((response) => setLeaderboard(response.entries))
            .catch(() => { });
    }, [room?.id]);
    useEffect(() => {
        if (!room?.id)
            return;
        refreshLeaderboard();
        const interval = setInterval(refreshLeaderboard, 2000);
        return () => clearInterval(interval);
    }, [room?.id, refreshLeaderboard]);
    const handleUser = (newUser) => {
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
        if (!room?.id)
            return;
        try {
            setLoadingAction("start");
            const response = await api.startGame(room.id);
            setRoom(response.room);
            toast.success("Partida iniciada");
        }
        catch {
            toast.error("No se pudo iniciar la partida");
        }
        finally {
            setLoadingAction(null);
        }
    };
    const handleGenerateQuestion = async () => {
        if (!room?.id)
            return;
        try {
            setLoadingAction("question");
            const response = await api.generateQuestion(room.id);
            setQuestion(response.question);
            toast.success("Pregunta generada");
        }
        catch {
            toast.error("No se pudo generar la pregunta");
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
            toast.success("Partida finalizada");
        }
        catch {
            toast.error("No se pudo finalizar la partida");
        }
        finally {
            setLoadingAction(null);
        }
    };
    const handleAnswer = async (option) => {
        if (!room?.id || !user?.id || !question?.id)
            return;
        try {
            const response = await api.submitAnswer(room.id, user.id, question.id, option, question.correctOption);
            if (response.correct) {
                confetti({
                    particleCount: 120,
                    spread: 75,
                    origin: { y: 0.65 },
                });
                toast.success("+100 puntos");
            }
            else {
                toast.error("Respuesta incorrecta");
            }
            refreshLeaderboard();
        }
        catch {
            toast.error("No se pudo enviar la respuesta");
        }
    };
    const roomLabel = useMemo(() => {
        if (!room?.id)
            return "Sin sala activa";
        return room.id;
    }, [room?.id]);
    return (_jsxs("main", { className: "arena-shell", children: [_jsxs("section", { className: "arena-hero", children: [_jsxs("div", { children: [_jsx("span", { className: "arena-eyebrow", children: "UCAB Cloud Project" }), _jsx("h1", { children: "Anime Quiz Arena" }), _jsx("p", { children: "Trivia anime en tiempo real usando microservicios, salas, preguntas y ranking en vivo." })] }), user && (_jsxs("div", { className: "user-pill", children: [_jsx("span", { children: "Jugador" }), _jsx("strong", { children: user.email || user.id || "Lasb" }), _jsx("button", { type: "button", className: "ghost-button", onClick: handleLogout, children: "Salir" })] }))] }), !user ? (_jsx("section", { className: "arena-card auth-card", children: _jsx(LoginPanel, { onUser: handleUser }) })) : (_jsxs(_Fragment, { children: [_jsxs("section", { className: "arena-card lobby-card", children: [_jsxs("div", { className: "section-header", children: [_jsxs("div", { children: [_jsx("span", { className: "arena-eyebrow", children: "Lobby" }), _jsx("h2", { children: "Sala de juego" })] }), _jsxs("div", { className: "room-badge", children: [_jsx("span", { children: "Room" }), _jsx("strong", { children: roomLabel })] })] }), _jsx(Lobby, { user: user, onRoom: setRoom, room: room })] }), hasRoom && (_jsxs("section", { className: "action-bar", children: [_jsx("button", { type: "button", className: "success-button", disabled: loadingAction === "start", onClick: handleStartGame, children: loadingAction === "start" ? "Iniciando..." : "Iniciar partida" }), _jsx("button", { type: "button", className: "primary-button", disabled: loadingAction === "question", onClick: handleGenerateQuestion, children: loadingAction === "question"
                                    ? "Generando..."
                                    : "Generar pregunta" }), _jsx("button", { type: "button", className: "danger-button", disabled: loadingAction === "end", onClick: handleEndGame, children: loadingAction === "end"
                                    ? "Finalizando..."
                                    : "Finalizar partida" })] })), _jsxs("section", { className: "arena-grid", children: [_jsx("div", { className: "arena-card question-section", children: question ? (_jsx(QuestionCard, { question: question, onAnswer: handleAnswer })) : (_jsxs("div", { className: "empty-state", children: [_jsx("span", { children: "\uD83C\uDF8C" }), _jsx("h2", { children: "Sin pregunta activa" }), _jsx("p", { children: "Genera una pregunta para comenzar la ronda." })] })) }), _jsxs("div", { className: "arena-card leaderboard-section", children: [_jsxs("div", { className: "section-header", children: [_jsxs("div", { children: [_jsx("span", { className: "arena-eyebrow", children: "Ranking" }), _jsx("h2", { children: "Leaderboard" })] }), _jsx("button", { type: "button", className: "ghost-button", onClick: refreshLeaderboard, children: "Refrescar" })] }), _jsx(Leaderboard, { entries: leaderboard, onRefresh: refreshLeaderboard })] })] })] }))] }));
}
