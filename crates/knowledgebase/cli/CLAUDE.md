adi-knowledgebase-cli, rust, cli, knowledge-management

## Overview
- CLI interface for ADI Knowledgebase
- Commands: add, query, approve, clarify, conflicts, ask, show, delete, link, orphans, status

## Commands
- `kb add --user-said "..." <knowledge>` - Add knowledge with user statement
- `kb query <question>` - Search the knowledgebase
- `kb approve <node_id>` - Approve a node (confidence = 1.0)
- `kb clarify <node_id>` - Request clarification
- `kb conflicts` - Show conflicts
- `kb ask <question>` - System asks user a question
- `kb show <node_id>` - Show node details
- `kb delete <node_id>` - Delete a node
- `kb link <from> <to>` - Link two nodes
- `kb orphans` - Show orphan nodes
- `kb status` - Show status

## Dependencies
- `adi-knowledgebase-core` - Core library
- `lib-cli-common` - Common CLI utilities
- `clap` - CLI parsing
