import { LitElement, html } from 'lit';
import { state } from 'lit/decorators.js';
import type { BalanceResponse, BalanceTransactionResponse } from './types.js';
import { cocoon } from './cocoon.js';

export class AdiPaymentElement extends LitElement {
  @state() private balance: BalanceResponse | null = null;
  @state() private transactions: BalanceTransactionResponse[] = [];
  @state() private canCharge: boolean | null = null;
  @state() private loading = false;
  @state() private error: string | null = null;

  private unsubs: Array<() => void> = [];

  override createRenderRoot() { return this; }

  private get bus() { return cocoon.bus; }

  override connectedCallback() {
    super.connectedCallback();
    this.unsubs.push(
      this.bus.on('payment:data-changed', ({ balance, canCharge }) => {
        this.balance = balance;
        this.canCharge = canCharge;
        this.loading = false;
      }, 'payment-ui'),
      this.bus.on('payment:transactions-changed', ({ transactions }) => {
        this.transactions = transactions;
        this.loading = false;
      }, 'payment-ui'),
    );
    this.refresh();
  }

  override disconnectedCallback() {
    super.disconnectedCallback();
    this.unsubs.forEach(fn => fn());
    this.unsubs = [];
  }

  private refresh(): void {
    this.loading = true;
    this.error = null;
    this.bus.emit('payment:balance', {}, 'payment-ui');
    this.bus.emit('payment:transactions', {}, 'payment-ui');
  }

  private formatDate(iso: string): string {
    return new Date(iso).toLocaleString();
  }

  override render() {
    if (this.loading) {
      return html`<div style="padding:1rem">Loading...</div>`;
    }

    if (this.error) {
      return html`<div style="padding:1rem;color:var(--color-error,red)">${this.error}</div>`;
    }

    return html`
      <div style="padding:1rem;display:flex;flex-direction:column;gap:1.5rem">
        ${this.renderBalance()}
        ${this.renderTransactions()}
      </div>
    `;
  }

  private renderBalance() {
    if (!this.balance) return html``;

    return html`
      <div>
        <h2 style="margin:0 0 0.75rem">Balance</h2>
        <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:0.75rem">
          <div>
            <div style="font-size:0.85rem;opacity:0.7">Subscription</div>
            <div style="font-size:1.25rem;font-weight:600">${this.balance.subscription_credits}</div>
          </div>
          <div>
            <div style="font-size:0.85rem;opacity:0.7">Extra</div>
            <div style="font-size:1.25rem;font-weight:600">${this.balance.extra_credits}</div>
          </div>
          <div>
            <div style="font-size:0.85rem;opacity:0.7">Total</div>
            <div style="font-size:1.25rem;font-weight:600">${this.balance.total_credits}</div>
          </div>
          <div>
            <div style="font-size:0.85rem;opacity:0.7">Can charge</div>
            <div style="font-size:1.25rem;font-weight:600">${this.canCharge ? 'Yes' : 'No'}</div>
          </div>
        </div>
        <div style="margin-top:0.5rem;font-size:0.8rem;opacity:0.5">
          Updated ${this.formatDate(this.balance.updated_at)}
        </div>
      </div>
    `;
  }

  private renderTransactions() {
    if (this.transactions.length === 0) {
      return html`<div style="opacity:0.5">No transactions yet.</div>`;
    }

    return html`
      <div>
        <h2 style="margin:0 0 0.75rem">Transactions</h2>
        <table style="width:100%;border-collapse:collapse;font-size:0.9rem">
          <thead>
            <tr style="text-align:left;border-bottom:1px solid var(--color-border,#333)">
              <th style="padding:0.4rem 0.5rem">Type</th>
              <th style="padding:0.4rem 0.5rem">Pool</th>
              <th style="padding:0.4rem 0.5rem;text-align:right">Amount</th>
              <th style="padding:0.4rem 0.5rem">Description</th>
              <th style="padding:0.4rem 0.5rem">Date</th>
            </tr>
          </thead>
          <tbody>
            ${this.transactions.map(tx => html`
              <tr style="border-bottom:1px solid var(--color-border,#222)">
                <td style="padding:0.4rem 0.5rem">${tx.transaction_type}</td>
                <td style="padding:0.4rem 0.5rem">${tx.pool}</td>
                <td style="padding:0.4rem 0.5rem;text-align:right;color:${tx.amount >= 0 ? 'var(--color-success,green)' : 'var(--color-error,red)'}">
                  ${tx.amount >= 0 ? '+' : ''}${tx.amount}
                </td>
                <td style="padding:0.4rem 0.5rem;opacity:0.7">${tx.description ?? ''}</td>
                <td style="padding:0.4rem 0.5rem;opacity:0.7">${this.formatDate(tx.created_at)}</td>
              </tr>
            `)}
          </tbody>
        </table>
      </div>
    `;
  }
}
