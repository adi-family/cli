import { AdiPlugin } from '@adi-family/sdk-plugin';
import { AdiRouterBusKey } from '@adi/router-web-plugin/bus';
import type { AdiRouterChangedEvent } from '@adi/router-web-plugin/bus';
import { AdiDebugScreenBusKey } from './bus';
import type { AdiDebugScreenRegisterSectionEvent } from './bus';
import type { AdiDebugPageElement, DebugSection } from './component.js';
import { PLUGIN_ID } from './config';
import './bus';

const ROUTE_PREFIX = `/${PLUGIN_ID}`;

function pluginIdFromPath(path: string): string {
  if (!path.startsWith(ROUTE_PREFIX + '/')) return '';
  return path.slice(ROUTE_PREFIX.length + 1).split('/')[0];
}

export class DebugScreenPlugin extends AdiPlugin {
  readonly id = PLUGIN_ID;
  readonly version = '0.1.0';

  private el: AdiDebugPageElement | null = null;
  private sections: DebugSection[] = [];

  override async onRegister(): Promise<void> {
    await import('./component.js');

    this.el = document.createElement('adi-debug-page') as AdiDebugPageElement;
    this.el.onNavigate = (pluginId) => {
      this.bus.emit(
        AdiRouterBusKey.Navigate,
        { path: `${ROUTE_PREFIX}/${pluginId}` },
        PLUGIN_ID,
      );
    };

    this.bus.emit(
      AdiRouterBusKey.RegisterRoute,
      { pluginId: PLUGIN_ID, path: '', init: () => this.el!, label: 'Debug' },
      PLUGIN_ID,
    );

    this.bus.on(
      AdiDebugScreenBusKey.RegisterSection,
      ({ pluginId, init, label }: AdiDebugScreenRegisterSectionEvent) => {
        if (this.sections.some((s) => s.pluginId === pluginId)) return;
        this.sections = [...this.sections, { pluginId, init: init as () => HTMLElement, label }];
        this.syncElement();
      },
      PLUGIN_ID,
    );

    this.bus.on(
      AdiRouterBusKey.Changed,
      ({ path }: AdiRouterChangedEvent) => {
        if (!path.startsWith(ROUTE_PREFIX)) return;
        this.syncActivePlugin(path);
      },
      PLUGIN_ID,
    );

    this.syncActivePlugin(window.location.pathname);
  }

  override onUnregister(): void {
    this.el = null;
    this.sections = [];
  }

  private syncElement(): void {
    if (!this.el) return;
    this.el.sections = this.sections;
  }

  private syncActivePlugin(path: string): void {
    if (!this.el) return;
    const pluginId = pluginIdFromPath(path);
    this.el.activePluginId = pluginId || (this.sections[0]?.pluginId ?? '');
  }
}
