import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
export function Leaderboard({ entries, onRefresh, }) {
    return (_jsxs("div", { className: "card", children: [_jsx("p", { className: "eyebrow", children: "Ranking" }), _jsx("h2", { children: "Leaderboard" }), _jsx("button", { onClick: onRefresh, children: "Refresh" }), entries.length === 0 ? (_jsx("p", { className: "muted", children: "A\u00FAn no hay puntajes en esta sala." })) : (_jsx("table", { children: _jsx("tbody", { children: entries.map((e) => (_jsxs("tr", { children: [_jsxs("td", { children: ["#", e.rank] }), _jsx("td", { children: e.userId }), _jsxs("td", { children: [e.points, " pts"] })] }, e.userId))) }) })), _jsx("small", { children: "Actualizando ranking cada 2s..." })] }));
}
