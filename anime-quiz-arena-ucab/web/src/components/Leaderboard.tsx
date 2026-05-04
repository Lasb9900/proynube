import type { ScoreEntry } from "../types";

export function Leaderboard({ entries, onRefresh, playerNames = {} }: { entries: ScoreEntry[]; onRefresh: () => void; playerNames?: Record<string, string> }) {
  const rankLabel = (rank: number) => {
    if (rank === 1) return "🥇";
    if (rank === 2) return "🥈";
    if (rank === 3) return "🥉";
    return `#${rank}`;
  };

  return (
    <div className="card">
      <p className="eyebrow">Ranking</p>
      <h2>Leaderboard</h2>

      <button type="button" onClick={onRefresh}>
        Refresh
      </button>

      {entries.length === 0 ? (
        <p className="muted">Aún no hay puntajes en esta sala.</p>
      ) : (
        <table>
          <tbody>
            {entries.map((entry) => (
              <tr key={entry.userId}>
                <td>{rankLabel(entry.rank)}</td>
                <td>{playerNames[entry.userId] ?? entry.userId.slice(0, 8)}</td>
                <td>{entry.points} pts</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      <small>Actualizando ranking cada 2s...</small>
    </div>
  );
}
