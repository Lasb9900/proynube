import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
export function QuestionCard({ question, onAnswer, disabled = false, answered = false, selectedOption = null, correctOption, }) {
    const options = [
        { key: "A", text: question.optionA },
        { key: "B", text: question.optionB },
        { key: "C", text: question.optionC },
        { key: "D", text: question.optionD },
    ];
    return (_jsxs("div", { className: "card question-card", children: [_jsx("h3", { children: question.text }), _jsx("div", { className: "answers", children: options.map((option) => {
                    const isSelected = selectedOption === option.key;
                    const isCorrect = answered && correctOption === option.key;
                    return (_jsxs("button", { type: "button", className: [
                            "answer-option",
                            isSelected ? "selected-answer" : "",
                            isCorrect ? "correct-answer" : "",
                        ]
                            .filter(Boolean)
                            .join(" "), onClick: () => onAnswer(option.key), disabled: disabled || answered, children: [_jsxs("strong", { children: [option.key, ":"] }), " ", _jsx("span", { children: option.text || "Opcion no disponible" })] }, option.key));
                }) }), answered && (_jsx("p", { className: "muted answer-submitted", children: "Respuesta enviada. Esperando a los demas jugadores..." }))] }));
}
