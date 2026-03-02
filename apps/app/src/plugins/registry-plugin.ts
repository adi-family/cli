import { AdiPlugin } from '@adi-family/sdk-plugin';
import { RegistryHub } from '../app/registry-hub';
import type { AppContext } from '../app/app';

export class RegistryPlugin extends AdiPlugin {
  readonly id = 'app.registry';
  readonly version = '1.0.0';

  readonly hub: RegistryHub;
  private readonly ctx: AppContext;

  private constructor(ctx: AppContext) {
    super();
    this.ctx = ctx;
    this.hub = RegistryHub.init();
  }

  static init(ctx: AppContext): RegistryPlugin {
    return new RegistryPlugin(ctx);
  }

  override async onRegister(): Promise<void> {
    await this.hub.start({ db: this.ctx.db });
  }

  override onUnregister(): void {
    this.hub.dispose();
  }
}
