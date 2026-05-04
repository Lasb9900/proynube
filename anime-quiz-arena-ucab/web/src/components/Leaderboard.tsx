import type { ScoreEntry } from "../types";

export function Leaderboard({ entries, onRefresh, playerNames = {} }: { entries: ScoreEntry[]; onRefresh: () => void; playerNames?: Record<string, string> }) {
  const rankLabel = (rank: number) => (rank === 1 ? "🥇" : rank === 2 ? "🥈" : rank === 3 ? "🥉" : `#${rank}`);
  return (
    <div className="card">
      <p className="eyebrow">Ranking</p>
      <h2>Leaderboard</h2>
      <button onClick={onRefresh}>Refresh</button>
      {entries.length === 0 ? <p className="muted">Aun no hay puntajes en esta sala.</p> : (
        <table><tbody>{entries.map((e) => <tr key={e.userId}><td>{rankLabel(e.rank)}</td><td>{playerNames[e.userId] ?? e.userId.slice(0, 8)}</td><td>{e.points} pts</td></tr>)}</tbody></table>
      )}
      <small>Actualizando ranking cada 2s...</small>
    </div>
  );
}
