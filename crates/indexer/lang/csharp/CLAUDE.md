# adi-lang-csharp

C# language support plugin for ADI indexer.

## Features

- Full C# syntax parsing via tree-sitter-c-sharp
- Symbol extraction: classes, structs, interfaces, methods, properties
- Reference tracking: method calls, type references, using directives
- Visibility detection: public, private, protected, internal

## Supported Constructs

| Symbol Type | Description |
|-------------|-------------|
| Class | Class definitions with inheritance |
| Struct | Struct definitions |
| Interface | Interface definitions |
| Method | Instance and static methods |
| Property | Properties with getters/setters |
| Field | Instance and static fields |
| Enum | Enum types and values |
| Delegate | Delegate type definitions |
| Event | Event declarations |

## File Extensions

- `.cs`

## Usage

This plugin is automatically loaded by the ADI indexer when C# files are detected.

```bash
adi index --path ./src
adi search "user repository"
```
