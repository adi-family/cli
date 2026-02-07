/**
 * Agent Loop HTTP API
 *
 * Simple HTTP interface for running autonomous LLM agents.
 */

import "@typespec/http";

using TypeSpec.Http;

namespace AgentLoop;

// -- Models --

model RunRequest {
  task: string;
  maxIterations?: int32;
  systemPrompt?: string;
}

model RunResponse {
  success: boolean;
  response?: string;
  error?: string;
}

model StatusResponse {
  initialized: boolean;
  version: string;
}

// -- Interface --

@route("/api")
interface AgentLoopService {
  @get
  @route("/status")
  getStatus(): {
    @statusCode statusCode: 200;
    @body body: StatusResponse;
  };

  @post
  @route("/run")
  run(@body body: RunRequest): {
    @statusCode statusCode: 200;
    @body body: RunResponse;
  };
}
