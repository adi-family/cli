import type {
  Connection,
  BalanceResponse,
  CanChargeMoreResponse,
  BalanceTransactionResponse,
} from './types.js';

const SVC = 'payment';

const proxyGet = async <T>(c: Connection, path: string): Promise<T> => {
  const res = await c.httpProxy(SVC, path);
  if (!res.ok) throw new Error(`Payment API error: ${res.status}`);
  return res.json() as Promise<T>;
};

export const getBalance = (c: Connection) =>
  proxyGet<BalanceResponse>(c, '/balance');

export const canChargeMore = (c: Connection) =>
  proxyGet<CanChargeMoreResponse>(c, '/balance/can-charge-more');

export const listTransactions = (c: Connection) =>
  proxyGet<BalanceTransactionResponse[]>(c, '/balance/transactions');
