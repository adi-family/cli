import { AdiPlugin } from '@adi-family/sdk-plugin';
import { SignalingHub } from '../app/signaling-hub';
import type { AppContext } from '../app/app';

export class SignalingPlugin extends AdiPlugin {
  readonly id = 'app.signaling';
  readonly version = '1.0.0';

  readonly hub: SignalingHub;
  private readonly ctx: AppContext;

  private constructor(ctx: AppContext) {
    super();
    this.ctx = ctx;
    this.hub = SignalingHub.init(ctx.bus);
  }

  static init(ctx: AppContext): SignalingPlugin {
    return new SignalingPlugin(ctx);
  }

  override async onRegister(): Promise<void> {
    await this.hub.start({ db: this.ctx.db });
  }

  override onUnregister(): void {
    this.hub.dispose();
  }
}
