# adi-lang-php

PHP language support plugin for ADI indexer.

## Features

- Full PHP syntax parsing via tree-sitter-php
- Symbol extraction: classes, interfaces, traits, functions, methods
- Reference tracking: function calls, use statements, includes
- Visibility detection: public, private, protected

## Supported Constructs

| Symbol Type | Description |
|-------------|-------------|
| Class | Class definitions with inheritance |
| Interface | Interface definitions |
| Trait | Trait definitions |
| Function | Standalone functions |
| Method | Instance and static methods |
| Property | Class properties |
| Constant | Class and global constants |

## File Extensions

- `.php`
- `.phtml`

## Usage

This plugin is automatically loaded by the ADI indexer when PHP files are detected.

```bash
adi index --path ./src
adi search "database connection"
```
