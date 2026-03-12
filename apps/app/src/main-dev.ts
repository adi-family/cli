import './index.css';
import './components/app-root.ts';
import { App } from './app/app.ts';
import { devPlugins } from './dev-plugins.ts';

App.initDev(devPlugins).then((v) => v.start());
