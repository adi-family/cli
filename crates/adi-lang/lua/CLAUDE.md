# adi-lang-lua

Lua language support plugin for ADI indexer.

## Features

- Full Lua syntax parsing via tree-sitter-lua
- Symbol extraction: functions, tables, local variables
- Reference tracking: function calls, table access, requires
- Scope detection: local vs global definitions

## Supported Constructs

| Symbol Type | Description |
|-------------|-------------|
| Function | Named and anonymous functions |
| Table | Table definitions |
| Variable | Local and global variables |
| Field | Table fields and methods |

## File Extensions

- `.lua`

## Usage

This plugin is automatically loaded by the ADI indexer when Lua files are detected.

```bash
adi index --path ./scripts
adi search "game loop"
```
