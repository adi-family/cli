// src/main.ts
import './index.css';
import './components/app-root.ts';
import { App } from './app/app.ts';

App.init()
  .then((v) => v.start())
  .catch((err) => {
    window.dispatchEvent(
      new CustomEvent('loading-error', { detail: err.message }),
    );
  });
