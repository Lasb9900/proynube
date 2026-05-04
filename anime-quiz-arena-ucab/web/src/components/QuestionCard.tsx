import type { Question } from "../types";

export function QuestionCard({
  question,
  onAnswer,
}: {
  question: Question;
  onAnswer: (v: string) => void;
}) {
  const options = [
    { key: "A", text: question.optionA },
    { key: "B", text: question.optionB },
    { key: "C", text: question.optionC },
    { key: "D", text: question.optionD },
  ];

  return (
    <div className="card">
      <h3>{question.text}</h3>

      {options.map((option) => (
        <button key={option.key} onClick={() => onAnswer(option.key)}>
          <strong>{option.key}:</strong> {option.text || "Opción no disponible"}
        </button>
      ))}
    </div>
  );
}
