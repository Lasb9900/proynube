import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { useState } from 'react';
import { toast } from 'sonner';
import * as api from '../api';
export function LoginPanel({ onUser }) {
    const [isRegister, set] = useState(true);
    const [f, setF] = useState({ username: '', email: '', password: '' });
    const submit = async () => { try {
        const r = isRegister ? await api.createUser(f.username, f.email, f.password) : await api.login(f.email, f.password);
        onUser(r.user);
        toast.success('Sesión iniciada');
    }
    catch (e) {
        toast.error(e.message);
    } };
    return _jsxs("div", { className: 'card', children: [_jsx("h2", { children: isRegister ? 'Crear usuario' : 'Login' }), _jsx("input", { placeholder: 'username', onChange: e => setF({ ...f, username: e.target.value }) }), _jsx("input", { placeholder: 'email', onChange: e => setF({ ...f, email: e.target.value }) }), _jsx("input", { placeholder: 'password', type: 'password', onChange: e => setF({ ...f, password: e.target.value }) }), _jsx("button", { onClick: submit, children: isRegister ? 'Crear usuario' : 'Login' }), _jsx("button", { onClick: () => set(!isRegister), children: "Cambiar" })] });
}
