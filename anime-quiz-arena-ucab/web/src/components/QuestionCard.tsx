import type { Question } from "../types";

export function QuestionCard({ question, onAnswer, disabled, selectedOption, correctOption, answered }: { question: Question; onAnswer: (v: string) => void; disabled?: boolean; selectedOption?: string; correctOption?: string; answered?: boolean; }) {
  const options = [
    { key: "A", text: question.optionA },
    { key: "B", text: question.optionB },
    { key: "C", text: question.optionC },
    { key: "D", text: question.optionD },
  ];

  return <div className="card"><h3>{question.text}</h3>{answered && <p className="muted">Respuesta enviada</p>}
    {options.map((option) => {
      const isSelected = selectedOption === option.key;
      const isCorrect = answered && correctOption === option.key;
      return <button key={option.key} disabled={disabled || answered} className={`${isSelected ? "option-selected" : ""} ${isCorrect ? "option-correct" : ""}`.trim()} onClick={() => onAnswer(option.key)}><strong>{option.key}:</strong> {option.text || "Opción no disponible"}</button>;
    })}
  </div>;
}
