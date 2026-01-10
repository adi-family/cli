# lib-client-uzu

Rust client library for the [Uzu](https://github.com/trymirai/uzu) inference engine, optimized for running large language models on Apple Silicon.

## Features

- 🚀 **Apple Silicon Optimized**: Leverages Metal GPU and unified memory
- 🔒 **100% Local**: No network requests, fully offline inference
- 🎯 **Simple API**: Idiomatic Rust interface with builder pattern
- ⚡ **Fast**: ~35 tokens/sec on Apple M2 (Llama-3.2-1B)
- 🛡️ **Type-Safe**: Comprehensive error handling

## Requirements

- macOS with Apple Silicon (M1/M2/M3 or later)
- Xcode and Metal toolchain
- Rust 2021 edition or later

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
lib-client-uzu = { git = "https://github.com/adi-family/lib-client-uzu" }
```

## Quick Start

```rust
use lib_client_uzu::{Client, GenerateRequest};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load a model
    let mut client = Client::new("models/llama-3.2-1b.gguf")?;

    // Generate text
    let request = GenerateRequest::new("Explain Rust ownership in simple terms")
        .max_tokens(256)
        .temperature(0.7);

    let response = client.generate(request)?;
    println!("{}", response.text);

    Ok(())
}
```

## Usage Examples

### Basic Generation

```rust
use lib_client_uzu::{Client, GenerateRequest};

let mut client = Client::new("models/llama-3.2-1b.gguf")?;

let request = GenerateRequest::new("Tell me a joke")
    .max_tokens(100);

let response = client.generate(request)?;
println!("Response: {}", response.text);
println!("Tokens generated: {}", response.tokens_generated);
```

### With Temperature Control

```rust
// Higher temperature = more creative/random
let request = GenerateRequest::new("Write a creative story")
    .max_tokens(500)
    .temperature(0.9);

let response = client.generate(request)?;
```

### With Stop Sequences

```rust
// Stop generation when encountering specific sequences
let request = GenerateRequest::new("List three colors:")
    .max_tokens(100)
    .stop_sequence("\n\n")  // Stop at double newline
    .stop_sequence("4.");    // Stop at "4."

let response = client.generate(request)?;

if response.stopped {
    println!("Stopped due to: {:?}", response.stop_reason);
}
```

### Using the Builder Pattern

```rust
use lib_client_uzu::ClientBuilder;

let mut client = ClientBuilder::new()
    .model_path("models/qwen-2.5-1b.gguf")
    .build()?;

let response = client.generate(
    GenerateRequest::new("Hello!")
        .max_tokens(50)
)?;
```

### Model Information

```rust
let client = Client::new("models/llama-3.2-1b.gguf")?;

let info = client.model_info();
println!("Model: {}", info.name);
println!("Size: {} bytes", info.size.unwrap_or(0));
println!("Loaded: {}", info.loaded);
```

## Supported Models

Uzu supports various model formats. Recommended models for Apple Silicon:

- **Llama 3.2 1B/3B** - Fast, general purpose
- **Qwen 2.5 1B/3B** - Good for multilingual tasks
- **Gemma 2B** - Google's efficient model

Models should be in GGUF format or Uzu's native format.

## Performance

Benchmark results on Apple M2:

| Model | Tokens/Second |
|-------|---------------|
| Llama-3.2-1B | ~35 |
| Qwen-2.5-1B | ~33 |
| Gemma-2B | ~28 |

Performance is competitive with llama.cpp and optimized for Apple Silicon's unified memory architecture.

## Error Handling

```rust
use lib_client_uzu::{Client, UzuError};

match Client::new("models/nonexistent.gguf") {
    Ok(client) => println!("Model loaded"),
    Err(UzuError::ModelNotFound(path)) => {
        eprintln!("Model not found: {}", path);
    },
    Err(UzuError::ModelLoad(msg)) => {
        eprintln!("Failed to load model: {}", msg);
    },
    Err(e) => eprintln!("Error: {}", e),
}
```

## Comparison with Other Solutions

| Feature | lib-client-uzu | lib-client-ollama | lib-client-openai |
|---------|---------------|-------------------|-------------------|
| Network required | ❌ No | ✅ Yes (local server) | ✅ Yes (API) |
| Apple Silicon optimized | ✅ Yes | ⚠️ Partial | N/A |
| Async API | ❌ No | ✅ Yes | ✅ Yes |
| Privacy | ✅ 100% local | ✅ Local | ❌ Cloud |
| Setup complexity | Low | Medium | Low |
| Cost | Free | Free | Paid API |

**When to use lib-client-uzu:**
- You're on Apple Silicon (M1/M2/M3)
- You need maximum performance for local inference
- You want zero network dependencies
- You prefer a simple synchronous API

**When to use lib-client-ollama:**
- You need cross-platform support
- You want more model management features
- You prefer async/streaming APIs

## Integration with ADI

```rust
// Use in ADI agent loop for local inference
use lib_client_uzu::Client;

let mut llm = Client::new("models/llama-3.2-3b.gguf")?;

// Generate responses without API costs
let response = llm.generate(
    GenerateRequest::new("Analyze this code: fn main() {}")
        .max_tokens(500)
)?;
```

## Troubleshooting

### "Model not found" error

Ensure the model file exists at the specified path:
```bash
ls -lh models/llama-3.2-1b.gguf
```

### "Failed to load model" error

- Verify you're on Apple Silicon (M1/M2/M3)
- Ensure Xcode and Metal toolchain are installed
- Check model format is compatible (GGUF recommended)

### Performance issues

- Ensure model fits in memory
- Close other applications to free up RAM
- Try smaller models (1B/2B parameters)

## License

MIT

## Contributing

Contributions welcome! Please open an issue or PR on GitHub.

## Related Projects

- [Uzu](https://github.com/trymirai/uzu) - The underlying inference engine
- [lib-client-ollama](https://github.com/adi-family/lib-client-ollama) - Ollama client library
- [ADI](https://adi.the-ihor.com) - Agent development infrastructure
