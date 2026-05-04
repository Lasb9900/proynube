import { jsxs as _jsxs, jsx as _jsx } from "react/jsx-runtime";
import { useState } from "react";
import { toast } from "sonner";
import * as api from "../api";
export function Lobby({ user, onRoom, room, onRefreshRoomState }) {
    const [name, setName] = useState("UCAB Anime Arena");
    const [joinId, setJoin] = useState("");
    return _jsxs("div", { className: "card", children: [_jsxs("h3", { children: ["Lobby - ", user.username] }), _jsx("input", { value: name, onChange: (e) => setName(e.target.value) }), _jsx("button", { onClick: async () => { const r = await api.createRoom(name, user.id); onRoom(r.room); await api.joinRoom(r.room.id, user.id, user.username); onRefreshRoomState?.(); toast.success("Sala creada"); }, children: "Crear sala" }), _jsx("input", { placeholder: "room_id", value: joinId, onChange: (e) => setJoin(e.target.value) }), _jsx("button", { onClick: async () => { const r = await api.joinRoom(joinId, user.id, user.username); onRoom(r.room); onRefreshRoomState?.(); }, children: "Unirse" }), room && _jsxs("p", { children: ["Room: ", _jsx("b", { children: room.id })] })] });
}
