import type { BalanceResponse, CanChargeMoreResponse, BalanceTransactionResponse } from './types.js';

declare module '@adi-family/sdk-plugin' {
  interface EventRegistry {
    'payment:balance':           Record<string, never>;
    'payment:balance:ok':        { balance: BalanceResponse; _cid: string };

    'payment:can-charge-more':           Record<string, never>;
    'payment:can-charge-more:ok':        { allowed: boolean; _cid: string };

    'payment:transactions':      Record<string, never>;
    'payment:transactions:ok':   { transactions: BalanceTransactionResponse[]; _cid: string };
  }
}

export {};
