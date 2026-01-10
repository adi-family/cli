# adi-lang-ruby

Ruby language support plugin for ADI indexer.

## Features

- Full Ruby syntax parsing via tree-sitter-ruby
- Symbol extraction: classes, modules, methods, constants
- Reference tracking: method calls, require/require_relative, includes
- Visibility detection: public, private, protected

## Supported Constructs

| Symbol Type | Description |
|-------------|-------------|
| Class | Class definitions with inheritance |
| Module | Module definitions |
| Method | Instance and class methods |
| Constant | Constant definitions |
| Attr | attr_reader, attr_writer, attr_accessor |
| Block | Block definitions (do..end, {}) |

## File Extensions

- `.rb`
- `.rake`
- `Gemfile`
- `Rakefile`

## Usage

This plugin is automatically loaded by the ADI indexer when Ruby files are detected.

```bash
adi index --path ./app
adi search "user authentication"
```
