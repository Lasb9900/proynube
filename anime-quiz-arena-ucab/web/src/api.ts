const API_BASE_URL =
  import.meta.env.VITE_API_BASE_URL ?? "http://localhost:8080";

async function req(path: string, opts: RequestInit = {}) {
  try {
    const r = await fetch(`${API_BASE_URL}${path}`, {
      headers: { "Content-Type": "application/json", ...(opts.headers ?? {}) },
      ...opts,
    });

    const d = await r.json().catch(() => null);

    if (!r.ok) throw new Error(d?.error || "Error API");

    return d;
  } catch (e) {
    if (e instanceof TypeError) {
      throw new Error("No se pudo conectar al API Gateway");
    }
    throw e;
  }
}

function normalizeQuestion(q: any) {
  return {
    id: q.id ?? "",
    text: q.text ?? "",
    optionA: q.optionA ?? q.option_a ?? "",
    optionB: q.optionB ?? q.option_b ?? "",
    optionC: q.optionC ?? q.option_c ?? "",
    optionD: q.optionD ?? q.option_d ?? "",
    correctOption: q.correctOption ?? q.correct_option ?? "",
    animeId: q.animeId ?? q.anime_id ?? 0,
  };
}

function normalizeScoreEntry(e: any) {
  return {
    userId: e.userId ?? e.user_id ?? "",
    points: e.points ?? 0,
    rank: e.rank ?? 0,
  };
}

export const health = () => req("/health");

export const createUser = (username: string, email: string, password: string) =>
  req("/api/users", {
    method: "POST",
    body: JSON.stringify({ username, email, password }),
  });

export const login = (email: string, password: string) =>
  req("/api/login", {
    method: "POST",
    body: JSON.stringify({ email, password }),
  });

export async function generateQuestion(roomId: string) {
  const data = await req("/api/questions/generate", {
    method: "POST",
    body: JSON.stringify({ room_id: roomId }),
  });

  return {
    question: data.question ? normalizeQuestion(data.question) : null,
  };
}

export const searchAnime = (q: string) =>
  req(`/api/anime/search?q=${encodeURIComponent(q)}`);

export const createRoom = (name: string, createdBy: string) =>
  req("/api/rooms", {
    method: "POST",
    body: JSON.stringify({ name, created_by: createdBy }),
  });

export const joinRoom = (roomId: string, userId: string) =>
  req(`/api/rooms/${roomId}/join`, {
    method: "POST",
    body: JSON.stringify({ user_id: userId }),
  });

export const startGame = (roomId: string) =>
  req(`/api/rooms/${roomId}/start`, { method: "POST", body: "{}" });

export const submitAnswer = (
  roomId: string,
  userId: string,
  questionId: string,
  selectedOption: string,
  correctOption: string,
) =>
  req(`/api/rooms/${roomId}/answer`, {
    method: "POST",
    body: JSON.stringify({
      user_id: userId,
      question_id: questionId,
      selected_option: selectedOption,
      correct_option: correctOption,
    }),
  });

export async function getLeaderboard(roomId: string, limit = 10) {
  const data = await req(`/api/rooms/${roomId}/leaderboard?limit=${limit}`);

  return {
    entries: Array.isArray(data.entries)
      ? data.entries.map(normalizeScoreEntry)
      : [],
  };
}

export const endGame = (roomId: string) =>
  req(`/api/rooms/${roomId}/end`, { method: "POST", body: "{}" });
