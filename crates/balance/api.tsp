/**
 * Balance API
 *
 * Balance and transaction tracking for ADI platform credits.
 * Amounts are in microtokens (1 AdiToken = 1,000,000 microtokens).
 */

import "@typespec/http";

using TypeSpec.Http;

namespace Balance;

@format("uuid")
scalar uuid extends string;

// -- Enums --

enum TransactionType {
  deposit,
  debit,
  adjustment,
  transferIn: "transfer_in",
  transferOut: "transfer_out",
}

enum TransactionStatus {
  pending,
  completed,
  failed,
  reversed,
}

// -- Models --

model BalanceResponse {
  userId: uuid;
  amount: int64;
  amountFormatted: string;
  currency: string;
  updatedAt: utcDateTime;
}

model InitBalanceRequest {
  userId?: uuid;
}

model TransactionResponse {
  id: uuid;
  userId: uuid;
  transactionType: TransactionType;
  status: TransactionStatus;
  amount: int64;
  amountFormatted: string;
  balanceBefore: int64;
  balanceAfter: int64;
  description?: string;
  referenceType?: string;
  referenceId?: string;
  createdAt: utcDateTime;
}

model DepositRequest {
  userId: uuid;
  amount: int64;
  description?: string;
  referenceType?: string;
  referenceId?: string;
  idempotencyKey?: string;
  metadata?: Record<unknown>;
}

model DebitRequest {
  userId: uuid;
  amount: int64;
  description?: string;
  referenceType?: string;
  referenceId?: string;
  idempotencyKey?: string;
  metadata?: Record<unknown>;
}

model CheckBalanceRequest {
  userId: uuid;
  amount: int64;
}

model CheckBalanceResponse {
  sufficient: boolean;
  currentBalance: int64;
  requiredAmount: int64;
  shortfall?: int64;
}

model TransactionQuery {
  @query limit?: int64;
  @query offset?: int64;
  @query transactionType?: string;
  @query referenceType?: string;
}

// -- Interfaces --

@route("/balances")
interface BalanceService {
  @get
  @route("/me")
  getMyBalance(): {
    @statusCode statusCode: 200;
    @body body: BalanceResponse;
  };

  @post
  @route("/init")
  initBalance(@body body: InitBalanceRequest): {
    @statusCode statusCode: 201;
    @body body: BalanceResponse;
  };

  @get
  @route("/{userId}")
  getBalanceByUser(@path userId: uuid): {
    @statusCode statusCode: 200;
    @body body: BalanceResponse;
  };
}

@route("/transactions")
interface TransactionService {
  @get
  list(...TransactionQuery): {
    @statusCode statusCode: 200;
    @body body: TransactionResponse[];
  };

  @post
  @route("/deposit")
  deposit(@body body: DepositRequest): {
    @statusCode statusCode: 201;
    @body body: TransactionResponse;
  };

  @post
  @route("/debit")
  debit(@body body: DebitRequest): {
    @statusCode statusCode: 201;
    @body body: TransactionResponse;
  };

  @post
  @route("/check")
  checkBalance(@body body: CheckBalanceRequest): {
    @statusCode statusCode: 200;
    @body body: CheckBalanceResponse;
  };

  @get
  @route("/{id}")
  getTransaction(@path id: uuid): {
    @statusCode statusCode: 200;
    @body body: TransactionResponse;
  };
}
