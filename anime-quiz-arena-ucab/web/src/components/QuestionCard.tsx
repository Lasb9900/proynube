import type { Question } from "../types";

type QuestionCardProps = {
  question: Question;
  onAnswer: (option: string) => void;
  disabled?: boolean;
  answered?: boolean;
  selectedOption?: string | null;
  correctOption?: string;
};

export function QuestionCard({
  question,
  onAnswer,
  disabled = false,
  answered = false,
  selectedOption = null,
  correctOption,
}: QuestionCardProps) {
  const options = [
    { key: "A", text: question.optionA },
    { key: "B", text: question.optionB },
    { key: "C", text: question.optionC },
    { key: "D", text: question.optionD },
  ];

  return (
    <div className="card question-card">
      <h3>{question.text}</h3>

      <div className="answers">
        {options.map((option) => {
          const isSelected = selectedOption === option.key;
          const isCorrect = answered && correctOption === option.key;

          return (
            <button
              key={option.key}
              type="button"
              className={[
                "answer-option",
                isSelected ? "selected-answer" : "",
                isCorrect ? "correct-answer" : "",
              ]
                .filter(Boolean)
                .join(" ")}
              onClick={() => onAnswer(option.key)}
              disabled={disabled || answered}
            >
              <strong>{option.key}:</strong>{" "}
              <span>{option.text || "Opcion no disponible"}</span>
            </button>
          );
        })}
      </div>

      {answered && (
        <p className="muted answer-submitted">
          Respuesta enviada. Esperando a los demas jugadores...
        </p>
      )}
    </div>
  );
}
