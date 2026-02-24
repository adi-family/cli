/**
 * Analytics Ingestion HTTP API
 *
 * Receives analytics events via HTTP and persists to TimescaleDB.
 */

import "@typespec/http";

using TypeSpec.Http;

namespace AnalyticsIngestion;

// -- Models --

model EnrichedEvent {
  timestamp: utcDateTime;
  event: Record<unknown>;
  hostname?: string;
  environment?: string;
}

model BatchResponse {
  received: int32;
}

// -- Interface --

interface IngestionService {
  @post
  @route("/events/batch")
  ingestEvents(@body body: EnrichedEvent[]): {
    @statusCode statusCode: 200;
    @body body: BatchResponse;
  };
}
