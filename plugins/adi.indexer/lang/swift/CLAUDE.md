# adi-lang-swift

Swift language support plugin for ADI indexer.

## Features

- Full Swift syntax parsing via tree-sitter-swift
- Symbol extraction: classes, structs, enums, protocols, functions
- Reference tracking: function calls, type references, imports
- Access control detection: public, private, internal, fileprivate, open

## Supported Constructs

| Symbol Type | Description |
|-------------|-------------|
| Class | Class definitions with inheritance |
| Struct | Struct definitions |
| Enum | Enum with associated values |
| Protocol | Protocol definitions |
| Extension | Type extensions |
| Function | Standalone and member functions |
| Property | Stored and computed properties |
| Typealias | Type aliases |

## File Extensions

- `.swift`

## Usage

This plugin is automatically loaded by the ADI indexer when Swift files are detected.

```bash
adi index --path ./Sources
adi search "view controller"
```
