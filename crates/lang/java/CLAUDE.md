# adi-lang-java

Java language support plugin for ADI indexer.

## Features

- Full Java syntax parsing via tree-sitter-java
- Symbol extraction: classes, interfaces, methods, fields, enums
- Reference tracking: method calls, type references, imports
- Visibility detection: public, private, protected, package-private

## Supported Constructs

| Symbol Type | Description |
|-------------|-------------|
| Class | Class definitions with inheritance |
| Interface | Interface definitions |
| Method | Instance and static methods |
| Field | Instance and static fields |
| Enum | Enum types and values |
| Constructor | Class constructors |
| Annotation | Annotation definitions |

## File Extensions

- `.java`

## Usage

This plugin is automatically loaded by the ADI indexer when Java files are detected.

```bash
adi index --path ./src
adi search "authentication handler"
```
