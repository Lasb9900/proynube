import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
export function QuestionCard({ question, onAnswer, }) {
    const options = [
        { key: "A", text: question.optionA },
        { key: "B", text: question.optionB },
        { key: "C", text: question.optionC },
        { key: "D", text: question.optionD },
    ];
    return (_jsxs("div", { className: "card", children: [_jsx("h3", { children: question.text }), options.map((option) => (_jsxs("button", { onClick: () => onAnswer(option.key), children: [_jsxs("strong", { children: [option.key, ":"] }), " ", option.text || "Opción no disponible"] }, option.key)))] }));
}
