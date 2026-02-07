/**
 * Logging Service HTTP API
 *
 * Centralized log ingestion and querying with distributed tracing.
 * Stores logs in TimescaleDB with correlation IDs for cocoon, user, session, and hive.
 */

import "@typespec/http";

using TypeSpec.Http;

namespace Logging;

@format("uuid")
scalar uuid extends string;

// -- Enums --

enum LogLevel {
  trace,
  debug,
  info,
  notice,
  warn,
  error,
  fatal,
}

// -- Models --

model ErrorInfo {
  kind: string;
  message: string;
  stackTrace?: string;
}

model LogEntry {
  timestamp: utcDateTime;
  service: string;
  hostname?: string;
  environment?: string;
  version?: string;
  level: LogLevel;
  message: string;
  traceId: uuid;
  spanId: uuid;
  parentSpanId?: uuid;
  cocoonId?: string;
  userId?: string;
  sessionId?: string;
  hiveId?: string;
  fields?: Record<unknown>;
  error?: ErrorInfo;
  source?: string;
  target?: string;
}

model LogResponse {
  id: int64;
  timestamp: utcDateTime;
  service: string;
  hostname?: string;
  environment?: string;
  level: string;
  message: string;
  traceId: uuid;
  spanId: uuid;
  parentSpanId?: uuid;
  cocoonId?: string;
  userId?: string;
  sessionId?: string;
  hiveId?: string;
  fields?: Record<unknown>;
  errorKind?: string;
  errorMessage?: string;
  source?: string;
  target?: string;
}

model IngestResponse {
  received: int64;
}

model LogQueryResult {
  logs: LogResponse[];
  count: int64;
  limit: int64;
  offset: int64;
}

model TraceLogsResult {
  traceId: uuid;
  logs: LogResponse[];
  count: int64;
}

model SpanLogsResult {
  spanId: uuid;
  logs: LogResponse[];
  count: int64;
}

model CorrelationLogsResult {
  logs: LogResponse[];
  count: int64;
  limit: int64;
  offset: int64;
}

model ServiceStats {
  service: string;
  logCount: int64;
  errorCount: int64;
}

model StatsResponse {
  period: string;
  totalLogs: int64;
  servicesCount: int64;
  tracesCount: int64;
  errorCount: int64;
  warnCount: int64;
  byService: ServiceStats[];
}

model LogQueryParams {
  @query service?: string;
  @query level?: string;
  @query traceId?: uuid;
  @query cocoonId?: string;
  @query userId?: string;
  @query sessionId?: string;
  @query hiveId?: string;
  @query search?: string;
  @query from?: utcDateTime;
  @query to?: utcDateTime;
  @query limit?: int64;
  @query offset?: int64;
}

model PaginationParams {
  @query from?: utcDateTime;
  @query to?: utcDateTime;
  @query limit?: int64;
  @query offset?: int64;
}

// -- Interfaces --

@route("/logs")
interface LogService {
  @post
  @route("/batch")
  ingest(@body body: LogEntry[]): {
    @statusCode statusCode: 200;
    @body body: IngestResponse;
  };

  @get
  query(...LogQueryParams): {
    @statusCode statusCode: 200;
    @body body: LogQueryResult;
  };

  @get
  @route("/trace/{traceId}")
  getTraceLogs(@path traceId: uuid): {
    @statusCode statusCode: 200;
    @body body: TraceLogsResult;
  };

  @get
  @route("/span/{spanId}")
  getSpanLogs(@path spanId: uuid): {
    @statusCode statusCode: 200;
    @body body: SpanLogsResult;
  };

  @get
  @route("/cocoon/{cocoonId}")
  getCocoonLogs(@path cocoonId: string, ...PaginationParams): {
    @statusCode statusCode: 200;
    @body body: CorrelationLogsResult;
  };

  @get
  @route("/user/{userId}")
  getUserLogs(@path userId: string, ...PaginationParams): {
    @statusCode statusCode: 200;
    @body body: CorrelationLogsResult;
  };

  @get
  @route("/session/{sessionId}")
  getSessionLogs(@path sessionId: string, ...PaginationParams): {
    @statusCode statusCode: 200;
    @body body: CorrelationLogsResult;
  };

  @get
  @route("/stats")
  getStats(): {
    @statusCode statusCode: 200;
    @body body: StatsResponse;
  };
}
