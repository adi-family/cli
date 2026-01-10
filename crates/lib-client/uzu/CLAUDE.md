uzu-client, llm, inference, apple-silicon, local-models

## Overview
- Rust client library for Uzu inference engine
- Optimized for Apple Silicon (M1/M2/M3)
- Synchronous (blocking) API for local LLM execution
- No HTTP/network - pure local inference
- License: MIT

## Architecture
- **Client**: Wrapper around uzu::Session with builder pattern
- **GenerateRequest**: Configuration for text generation (prompt, max_tokens, temperature, etc.)
- **GenerateResponse**: Generated text with metadata (tokens_generated, stop_reason)
- **ModelInfo**: Model metadata (name, size, loaded status)
- **UzuError**: Typed error handling (ModelNotFound, InferenceError, etc.)

## Key Features
- Builder pattern for client construction
- Model file validation before loading
- Configurable generation parameters (max_tokens, temperature, top_p)
- Stop sequence support
- Tracing instrumentation for debugging
- Comprehensive error types

## Usage Pattern
```rust
// Load model
let mut client = Client::new("models/llama-3.2-1b.gguf")?;

// Generate text
let request = GenerateRequest::new("Prompt")
    .max_tokens(256)
    .temperature(0.7)
    .stop_sequence("\n");
let response = client.generate(request)?;
```

## Performance
- Apple M2: ~35 tokens/sec (Llama-3.2-1B)
- Competitive with llama.cpp on Apple Silicon
- Leverages Metal GPU acceleration
- Uses unified memory architecture

## Requirements
- macOS with Apple Silicon
- Xcode and Metal toolchain installed
- Model files in GGUF or native Uzu format

## Error Handling
- `ModelNotFound`: Model file doesn't exist
- `ModelLoad`: Failed to initialize model
- `InferenceError`: Runtime generation failure
- `InvalidConfig`: Missing required configuration
- `InvalidInput`: Malformed request

## Integration Points
- Can be used in ADI agent-loop for local inference
- Alternative to lib-client-ollama for Apple Silicon users
- Suitable for privacy-focused applications (no network)
- Good for development/testing without API costs

## Limitations
- Apple Silicon only (macOS M1/M2/M3+)
- Synchronous API (blocks during generation)
- No streaming support yet
- Model must fit in memory
