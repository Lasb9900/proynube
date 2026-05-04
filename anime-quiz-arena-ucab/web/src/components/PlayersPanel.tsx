import type { RoomState } from "../types";

export function PlayersPanel({ roomState, currentUserId }: { roomState: RoomState | null; currentUserId?: string }) {
  const total = roomState?.totalPlayers ?? 0;
  const answered = roomState?.answeredCount ?? 0;
  const progress = total > 0 ? Math.round((answered / total) * 100) : 0;

  return (
    <div className="card">
      <h3>Jugadores en sala</h3>
      <p className="muted">{answered}/{total} respondieron</p>
      <div className="progress-track"><div className="progress-fill" style={{ width: `${progress}%` }} /></div>
      <ul className="players-list">
        {(roomState?.players ?? []).map((player) => {
          const name = player.username || player.userId.slice(0, 8);
          const isMe = player.userId === currentUserId;
          return (
            <li key={player.userId}>
              <strong>{name}{isMe ? " (Tú)" : ""}</strong>
              <span>{player.answeredCurrentQuestion ? "✅ Respondio" : "🟡 Pendiente"}</span>
              <small>{player.ready ? "✅ Listo" : ""}</small>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
