import { useState } from "react";
import { toast } from "sonner";

import * as api from "../api";

type LobbyProps = {
  user: { id: string; username: string };
  room: { id: string; status?: string } | null;
  onRoom: (room: any) => void;
  onRefreshRoomState?: () => void;
};

export function Lobby({ user, room, onRoom, onRefreshRoomState }: LobbyProps) {
  const [roomName, setRoomName] = useState("UCAB Anime Arena");
  const [joinRoomId, setJoinRoomId] = useState("");

  const handleCreateRoom = async () => {
    try {
      const created = await api.createRoom(roomName, user.id);
      onRoom(created.room);

      await api.joinRoom(created.room.id, user.id, user.username);
      await onRefreshRoomState?.();

      toast.success("Sala creada");
    } catch {
      toast.error("No se pudo crear la sala");
    }
  };

  const handleJoinRoom = async () => {
    if (!joinRoomId.trim()) {
      toast.error("Ingresa un codigo de sala");
      return;
    }

    try {
      const joined = await api.joinRoom(joinRoomId.trim(), user.id, user.username);
      onRoom(joined.room);
      await onRefreshRoomState?.();
      toast.success("Te uniste a la sala");
    } catch {
      toast.error("No se pudo unir a la sala");
    }
  };

  const handleCopyRoomCode = async () => {
    if (!room?.id) return;

    try {
      await navigator.clipboard.writeText(room.id);
      toast.success("Codigo de sala copiado");
    } catch {
      toast.error("No se pudo copiar el codigo");
    }
  };

  return (
    <div className="card lobby-panel">
      <h3>Lobby - {user.username}</h3>

      <label htmlFor="room-name">Crear sala</label>
      <input
        id="room-name"
        value={roomName}
        onChange={(event) => setRoomName(event.target.value)}
      />
      <button type="button" onClick={handleCreateRoom}>
        Crear sala
      </button>

      <label htmlFor="join-room-id">Unirse a sala</label>
      <input
        id="join-room-id"
        placeholder="Ingresa room_id"
        value={joinRoomId}
        onChange={(event) => setJoinRoomId(event.target.value)}
      />
      <button type="button" onClick={handleJoinRoom}>
        Unirse a sala
      </button>

      {room?.id && (
        <div className="room-share-box">
          <p>
            Codigo de sala: <b>{room.id}</b>
          </p>
          <p className="muted">Comparte este codigo con otros jugadores</p>
          <button type="button" className="ghost-button" onClick={handleCopyRoomCode}>
            Copiar codigo de sala
          </button>
        </div>
      )}
    </div>
  );
}
