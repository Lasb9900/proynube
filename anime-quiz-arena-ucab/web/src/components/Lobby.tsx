import { useState } from "react";
import { toast } from "sonner";
import * as api from "../api";
export function Lobby({ user, onRoom, room, onRefreshRoomState }: { user: any; onRoom: (r: any) => void; room: any; onRefreshRoomState?: () => void }) {
  const [name, setName] = useState("UCAB Anime Arena"); const [joinId, setJoin] = useState("");
  return <div className="card"><h3>Lobby - {user.username}</h3><input value={name} onChange={(e) => setName(e.target.value)} /><button onClick={async()=>{const r=await api.createRoom(name,user.id); onRoom(r.room); await api.joinRoom(r.room.id,user.id,user.username); onRefreshRoomState?.(); toast.success("Sala creada");}}>Crear sala</button><input placeholder="room_id" value={joinId} onChange={(e)=>setJoin(e.target.value)} /><button onClick={async()=>{const r=await api.joinRoom(joinId,user.id,user.username); onRoom(r.room); onRefreshRoomState?.();}}>Unirse</button>{room&&<p>Room: <b>{room.id}</b></p>}</div>;
}
