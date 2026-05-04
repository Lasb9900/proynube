import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
export function Leaderboard({ entries, onRefresh, playerNames = {} }) {
    const rankLabel = (rank) => (rank === 1 ? "🥇" : rank === 2 ? "🥈" : rank === 3 ? "🥉" : `#${rank}`);
    return (_jsxs("div", { className: "card", children: [_jsx("p", { className: "eyebrow", children: "Ranking" }), _jsx("h2", { children: "Leaderboard" }), _jsx("button", { onClick: onRefresh, children: "Refresh" }), entries.length === 0 ? _jsx("p", { className: "muted", children: "Aun no hay puntajes en esta sala." }) : (_jsx("table", { children: _jsx("tbody", { children: entries.map((e) => _jsxs("tr", { children: [_jsx("td", { children: rankLabel(e.rank) }), _jsx("td", { children: playerNames[e.userId] ?? e.userId.slice(0, 8) }), _jsxs("td", { children: [e.points, " pts"] })] }, e.userId)) }) })), _jsx("small", { children: "Actualizando ranking cada 2s..." })] }));
}
