import { AdiPlugin } from '@adi-family/sdk-plugin';
import { AdiRouterBusKey } from '@adi/router-web-plugin/bus';
import * as api from './api.js';
import type { Connection } from './types.js';
import './events.js';

const pickConnection = (): Connection => {
  const connections = [...window.sdk.getConnections().values()];
  if (connections.length === 0) throw new Error('No connection available');
  return connections[0]!;
};

export class PaymentPlugin extends AdiPlugin {
  readonly id = 'adi.payment-web';
  readonly version = '0.1.0';

  async onRegister(): Promise<void> {
    const { AdiPaymentElement } = await import('./component.js');
    if (!customElements.get('adi-payment')) {
      customElements.define('adi-payment', AdiPaymentElement);
    }

    this.bus.emit(AdiRouterBusKey.RegisterRoute, { pluginId: this.id, path: '', init: () => document.createElement('adi-payment'), label: 'Payment' }, this.id);
    this.bus.emit('nav:add', { id: this.id, label: 'Payment', path: `/${this.id}` }, this.id);

    this.bus.on('payment:balance', async () => {
      try {
        const [balance, { allowed }] = await Promise.all([
          api.getBalance(pickConnection()),
          api.canChargeMore(pickConnection()),
        ]);
        this.bus.emit('payment:data-changed', { balance, canCharge: allowed }, 'payment');
      } catch (err) {
        console.error('[PaymentPlugin] payment:balance error:', err);
      }
    }, 'payment');

    this.bus.on('payment:can-charge-more', async () => {
      try {
        const [balance, { allowed }] = await Promise.all([
          api.getBalance(pickConnection()),
          api.canChargeMore(pickConnection()),
        ]);
        this.bus.emit('payment:data-changed', { balance, canCharge: allowed }, 'payment');
      } catch (err) {
        console.error('[PaymentPlugin] payment:can-charge-more error:', err);
      }
    }, 'payment');

    this.bus.on('payment:transactions', async () => {
      try {
        const transactions = await api.listTransactions(pickConnection());
        this.bus.emit('payment:transactions-changed', { transactions }, 'payment');
      } catch (err) {
        console.error('[PaymentPlugin] payment:transactions error:', err);
        this.bus.emit('payment:transactions-changed', { transactions: [] }, 'payment');
      }
    }, 'payment');
  }
}
