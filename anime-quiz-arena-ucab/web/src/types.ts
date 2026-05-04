export type User = {
  id: string;
  username: string;
  email: string;
  createdAt: string;
};

export type Room = {
  id: string;
  name: string;
  status: string;
  createdBy: string;
  createdAt: string;
};

export type Question = {
  id: string;
  text: string;
  optionA: string;
  optionB: string;
  optionC: string;
  optionD: string;
  correctOption: string;
  animeId: number;
};

export type ScoreEntry = {
  userId: string;
  points: number;
  rank: number;
};


export type RoomPlayer = {
  userId: string;
  username: string;
  answeredCurrentQuestion: boolean;
  ready: boolean;
  joinedAt: string;
};

export type RoomState = {
  room: Room | null;
  players: RoomPlayer[];
  totalPlayers: number;
  answeredCount: number;
  allAnswered: boolean;
};
