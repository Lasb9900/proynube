import type { Question } from '../types';
export function QuestionCard({question,onAnswer}:{question:Question,onAnswer:(v:string)=>void}){return <div className='card'><h3>{question.text}</h3>{(['A','B','C','D'] as const).map((k)=> <button key={k} onClick={()=>onAnswer(k)}>{k}: {(question as any)[`option_${k.toLowerCase()}`]}</button>)}</div>}
