const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080';
async function req(path:string, opts:RequestInit={}){ try{ const r=await fetch(`${API_BASE_URL}${path}`,{headers:{'Content-Type':'application/json'},...opts}); const d=await r.json(); if(!r.ok) throw new Error(d.error||'Error API'); return d;}catch(e){ if(e instanceof TypeError) throw new Error('No se pudo conectar al API Gateway'); throw e; } }
export const health=()=>req('/health');
export const createUser=(username:string,email:string,password:string)=>req('/api/users',{method:'POST',body:JSON.stringify({username,email,password})});
export const login=(email:string,password:string)=>req('/api/login',{method:'POST',body:JSON.stringify({email,password})});
export const generateQuestion=(room_id:string)=>req('/api/questions/generate',{method:'POST',body:JSON.stringify({room_id})});
export const searchAnime=(q:string)=>req(`/api/anime/search?q=${encodeURIComponent(q)}`);
export const createRoom=(name:string,created_by:string)=>req('/api/rooms',{method:'POST',body:JSON.stringify({name,created_by})});
export const joinRoom=(roomId:string,user_id:string)=>req(`/api/rooms/${roomId}/join`,{method:'POST',body:JSON.stringify({user_id})});
export const startGame=(roomId:string)=>req(`/api/rooms/${roomId}/start`,{method:'POST',body:'{}'});
export const submitAnswer=(roomId:string,user_id:string,question_id:string,selected_option:string,correct_option:string)=>req(`/api/rooms/${roomId}/answer`,{method:'POST',body:JSON.stringify({user_id,question_id,selected_option,correct_option})});
export const getLeaderboard=(roomId:string,limit=10)=>req(`/api/rooms/${roomId}/leaderboard?limit=${limit}`);
export const endGame=(roomId:string)=>req(`/api/rooms/${roomId}/end`,{method:'POST',body:'{}'});
