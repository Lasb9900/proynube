import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
export function PlayersPanel({ roomState, currentUserId }) {
    const total = roomState?.totalPlayers ?? 0;
    const answered = roomState?.answeredCount ?? 0;
    const progress = total > 0 ? Math.round((answered / total) * 100) : 0;
    return (_jsxs("div", { className: "card", children: [_jsx("h3", { children: "Jugadores en sala" }), _jsxs("p", { className: "muted", children: [answered, "/", total, " respondieron"] }), _jsx("div", { className: "progress-track", children: _jsx("div", { className: "progress-fill", style: { width: `${progress}%` } }) }), _jsx("ul", { className: "players-list", children: (roomState?.players ?? []).map((player) => {
                    const name = player.username || player.userId.slice(0, 8);
                    const isMe = player.userId === currentUserId;
                    return (_jsxs("li", { children: [_jsxs("strong", { children: [name, isMe ? " (Tú)" : ""] }), _jsx("span", { children: player.answeredCurrentQuestion ? "✅ Respondio" : "🟡 Pendiente" }), _jsx("small", { children: player.ready ? "✅ Listo" : "" })] }, player.userId));
                }) })] }));
}
