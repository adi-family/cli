import type { BalanceResponse, BalanceTransactionResponse } from './types.js';

declare module '@adi-family/sdk-plugin/types' {
  interface EventRegistry {
    'payment:balance':         Record<string, never>;
    'payment:can-charge-more': Record<string, never>;
    'payment:transactions':    Record<string, never>;

    'payment:data-changed':         { balance: BalanceResponse; canCharge: boolean };
    'payment:transactions-changed': { transactions: BalanceTransactionResponse[] };
  }
}

export {};
