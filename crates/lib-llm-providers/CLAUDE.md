lib-llm-providers, rust, llm, providers, routing, anthropic, openai, ollama

## Overview
- LLM provider implementations for adi-agent-loop-core
- Supports Claude (Anthropic), OpenAI, Ollama (local)
- Routing/orchestration patterns for multi-model setups

## Providers
- `ClaudeProvider` - Anthropic Claude API (claude-sonnet-4, opus, haiku)
- `OpenAiProvider` - OpenAI API (gpt-4o, gpt-4o-mini, o1, o3)
- `OllamaProvider` - Local Ollama server (llama, mistral, etc.)

## Routers
- `RouterProvider` - Route by config/function
- `FallbackProvider` - Try providers in order until success
- `CostAwareRouter` - Cheap model for simple, expensive for complex
- `LoadBalancedRouter` - Round-robin across providers

## Usage
```rust
use lib_llm_providers::{ClaudeProvider, OpenAiProvider, OllamaProvider};
use lib_llm_providers::{RouterProvider, FallbackProvider, CostAwareRouter};
use std::sync::Arc;

// Single provider
let claude = ClaudeProvider::new("sk-ant-...");
let openai = OpenAiProvider::new("sk-...");
let ollama = OllamaProvider::new().with_host("http://localhost:11434");

// Fallback chain
let fallback = FallbackProvider::new(vec![
    Arc::new(ClaudeProvider::new("key")),
    Arc::new(OpenAiProvider::new("key")),
]);

// Model-prefix router (auto-routes by model name)
let router = RouterProvider::new()
    .add_provider("claude", Arc::new(ClaudeProvider::new("key")))
    .add_provider("openai", Arc::new(OpenAiProvider::new("key")))
    .with_model_prefix_router();

// Cost-aware routing
let cost_aware = CostAwareRouter::new(
    Arc::new(ClaudeProvider::new("key")),  // cheap (haiku)
    Arc::new(ClaudeProvider::new("key")),  // expensive (opus)
).with_threshold(1000);
```
