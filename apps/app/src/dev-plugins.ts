import type { AdiPlugin } from '@adi-family/sdk-plugin';

// Core plugins (same initialization order as app.ts)
import { PluginShell as Slots } from '@adi-family/plugin-slots/build';
import { PluginShell as Router } from '@adi-family/plugin-router/build';
import { PluginShell as CommandPalette } from '@adi-family/plugin-command-palette/build';
import { PluginShell as Auth } from '@adi-family/plugin-auth/build';
import { PluginShell as DebugScreen } from '@adi-family/plugin-debug-screen/build';
import { PluginShell as Signaling } from '@adi-family/plugin-signaling/build';
import { PluginShell as Cocoon } from '@adi-family/plugin-cocoon/build';
import { PluginShell as ActionsFeed } from '@adi-family/plugin-actions-feed/build';
import { PluginShell as CocoonControlCenter } from '@adi-family/plugin-cocoon-control-center/build';
import { PluginShell as Credentials } from '@adi-family/plugin-credentials/build';
import { PluginShell as PluginsManager } from '@adi-family/plugin-plugins/build';

// Extended plugins
import { PluginShell as Payment } from '@adi-family/plugin-payment/build';
import { PluginShell as Tasks } from '@adi-family/plugin-tasks/build';
import { PluginShell as Knowledgebase } from '@adi-family/plugin-knowledgebase/build';
import { PluginShell as Video } from '@adi-family/plugin-video/build';
import { PluginShell as MonacoEditor } from '@adi-family/plugin-monaco-editor/build';

type PluginConstructor = new () => AdiPlugin;

export const devPlugins: PluginConstructor[] = [
  Slots,
  Router,
  CommandPalette,
  Auth,
  DebugScreen,
  Signaling,
  Cocoon,
  ActionsFeed,
  CocoonControlCenter,
  Credentials,
  PluginsManager,
  Payment,
  Tasks,
  Knowledgebase,
  Video,
  MonacoEditor,
];
