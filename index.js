import React from 'react';
import ReactDOM from 'react-dom/client';
import './index.css';
import App from './App.js';           // .js olarak değiştirin (jsx değil)
import reportWebVitals from './reportWebVitals.js';

const root = ReactDOM.createRoot(document.getElementById('root'));
root.render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);

reportWebVitals();