export interface Connection {
  id: string;
  services: string[];
  request<T>(service: string, method: string, params?: unknown): Promise<T>;
  httpProxy(service: string, path: string, init?: RequestInit): Promise<Response>;
  httpDirect(url: string, init?: RequestInit): Promise<Response>;
}

export interface BalanceResponse {
  subscription_credits: number;
  extra_credits: number;
  total_credits: number;
  updated_at: string;
}

export interface CanChargeMoreResponse {
  allowed: boolean;
}

export interface BalanceTransactionResponse {
  id: string;
  payment_id: string | null;
  transaction_type: string;
  pool: string;
  amount: number;
  balance_before: number;
  balance_after: number;
  conversion_rate: number;
  description: string | null;
  created_at: string;
}
