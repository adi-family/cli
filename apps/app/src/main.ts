// src/main.ts
import './index.css';
import './components/app-root.ts';
import { App } from './app/app.ts';

App.init().then((v) => {
  return v.start();
});
