import type { ScoreEntry } from "../types";

export function Leaderboard({
  entries,
  onRefresh,
}: {
  entries: ScoreEntry[];
  onRefresh: () => void;
}) {
  return (
    <div className="card">
      <p className="eyebrow">Ranking</p>
      <h2>Leaderboard</h2>

      <button onClick={onRefresh}>Refresh</button>

      {entries.length === 0 ? (
        <p className="muted">Aún no hay puntajes en esta sala.</p>
      ) : (
        <table>
          <tbody>
            {entries.map((e) => (
              <tr key={e.userId}>
                <td>#{e.rank}</td>
                <td>{e.userId}</td>
                <td>{e.points} pts</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      <small>Actualizando ranking cada 2s...</small>
    </div>
  );
}
