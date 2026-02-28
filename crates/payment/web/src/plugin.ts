import { AdiPlugin } from '@adi-family/sdk-plugin';
import type { WithCid } from '@adi-family/sdk-plugin';
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

    this.bus.emit('route:register', { path: '/payment', element: 'adi-payment' }, 'payment');
    this.bus.send('nav:add', { id: 'payment', label: 'Payment', path: '/payment' }, 'payment').handle(() => {});

    this.bus.emit('command:register', { id: 'payment:open', label: 'Go to Payment page' }, 'payment');
    this.bus.on('command:execute', ({ id }) => {
      if (id === 'payment:open') this.bus.emit('router:navigate', { path: '/payment' }, 'payment');
    }, 'payment');

    this.bus.on('payment:balance', async (p) => {
      const { _cid } = p as WithCid<typeof p>;
      try {
        const balance = await api.getBalance(pickConnection());
        this.bus.emit('payment:balance:ok', { balance, _cid }, 'payment');
      } catch (err) {
        console.error('[PaymentPlugin] payment:balance error:', err);
      }
    }, 'payment');

    this.bus.on('payment:can-charge-more', async (p) => {
      const { _cid } = p as WithCid<typeof p>;
      try {
        const result = await api.canChargeMore(pickConnection());
        this.bus.emit('payment:can-charge-more:ok', { allowed: result.allowed, _cid }, 'payment');
      } catch (err) {
        console.error('[PaymentPlugin] payment:can-charge-more error:', err);
      }
    }, 'payment');

    this.bus.on('payment:transactions', async (p) => {
      const { _cid } = p as WithCid<typeof p>;
      try {
        const transactions = await api.listTransactions(pickConnection());
        this.bus.emit('payment:transactions:ok', { transactions, _cid }, 'payment');
      } catch (err) {
        console.error('[PaymentPlugin] payment:transactions error:', err);
        this.bus.emit('payment:transactions:ok', { transactions: [], _cid }, 'payment');
      }
    }, 'payment');
  }
}
