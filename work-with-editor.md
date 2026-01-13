# Editor Components

## Editable Editors

### WebGL Editor (Main)
- **Path**: `apps/infra-service-web/src/app/[locale]/webgl-editor/page.tsx`
- **URL**: `http://adi.local/en/webgl-editor`
- **Type**: Full-page code editor with virtualized rendering
- **Technology**: Custom textarea overlay with syntax highlighting
- **Features**: Line numbers, current line highlight, 50K+ line support, One Dark theme
- **Languages**: Rust/JavaScript tokenizer

### RhaiEditor
- **Path**: `apps/infra-service-web/src/components/proxy/RhaiEditor.tsx`
- **Type**: Interactive code editor with syntax highlighting
- **Technology**: Custom textarea overlay technique
- **Features**: Rhai language tokenizer, scroll sync, focus management
- **Exports**: `RhaiEditor` (editable), `RhaiCode` (read-only)
- **Used by**: `TokensPanel.tsx` for LLM proxy script editing

## Read-Only Code Display

### CodeBlock
- **Path**: `apps/infra-service-web/src/components/code-block.tsx`
- **Type**: Generic syntax-highlighted code block
- **Technology**: `react-syntax-highlighter` with Prism (VS Code Dark Plus theme)
- **Used by**: `cocoon-install-commands.tsx`, `mdx-components.tsx`

### TypeSpecExample
- **Path**: `apps/infra-service-web/src/components/typespec-example.tsx`
- **Type**: Code showcase with language switching
- **Technology**: `react-syntax-highlighter` with Prism
- **Features**: Side-by-side TypeSpec + generated code, language switcher (Rust/Python/TypeScript)

## Backend Tokenizer

### lib-syntax-highlight
- **Path**: `crates/lib/lib-syntax-highlight/src/lib.rs`
- **Type**: Rust tokenizer library
- **Purpose**: Terminal/CLI output highlighting
- **Token types**: String, Number, Variable, Path, Key, Comment, Function, Boolean, URL, IP, DateTime, Error, Success
