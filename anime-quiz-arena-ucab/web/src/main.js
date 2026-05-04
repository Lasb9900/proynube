import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import React from 'react';
import ReactDOM from 'react-dom/client';
import { Toaster } from 'sonner';
import App from './App';
import './styles.css';
ReactDOM.createRoot(document.getElementById('root')).render(_jsxs(React.StrictMode, { children: [_jsx(Toaster, { richColors: true }), _jsx(App, {})] }));
