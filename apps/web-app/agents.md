web-app, lit, styling, tailwind, guidelines, stores, multi-source

## Styling Guidelines
- Prefer inline Tailwind classes over BEM naming conventions
- Use BEM only when truly necessary (complex component state management, third-party library integration)
- Keep styles co-located with components using Tailwind utility classes
- When writing CSS, use `@apply` with Tailwind utilities instead of raw CSS properties (e.g., `@apply w-full` not `width: 100%`)

## Icons
- Use Lucide icons for all iconography

## Store Architecture

### Overview
Multi-source reactive stores with offline-first sync using nanostores.

```
src/
  stores/
    core/
      types.ts           # Interfaces: Source, StoreItem, MultiSourceStore
      http-source.ts     # Generic HTTP adapter implementing Source<T>
      sync-queue.ts      # IndexedDB-backed offline mutation queue
      create-store.ts    # Factory: createMultiSourceStore<T>()
      index.ts           # Core exports
    credentials.ts       # Credentials store instance
    index.ts             # All store exports
  api/
    base-client.ts       # Shared HTTP client with error handling
```

### Key Concepts

**Multi-Source**: Each store can connect to multiple HTTP backends (local, cloud-prod, cloud-dev). Items from all sources are merged into a unified view, tagged with `_meta.source`.

**Reactive**: Stores expose nanostores atoms (`$items`, `$loading`, `$error`, etc.) for automatic UI updates via `StoreController`.

**Offline-First**: Mutations are queued in IndexedDB when offline and synced when connectivity is restored.

### Creating a New Store

```typescript
// src/stores/my-entity.ts
import { createMultiSourceStore } from './core';
import type { MyEntity } from '../models/my-entity';

export const myEntityStore = createMultiSourceStore<MyEntity>({
  name: 'my-entity',  // Used for IndexedDB keys
  sources: [
    { id: 'cloud', name: 'Cloud', baseUrl: '/api/my-entity', priority: 10 },
  ],
});

// Add more sources at runtime
myEntityStore.addSource({
  id: 'local',
  name: 'Local',
  baseUrl: 'http://localhost:8000/my-entity',
  priority: 5,
});
```

### Using Stores in Lit Components

```typescript
import { LitElement, html } from 'lit';
import { StoreController } from '@nanostores/lit';
import { myEntityStore } from '../stores/my-entity';

class MyComponent extends LitElement {
  // Reactive bindings - auto-update on store changes
  private items = new StoreController(this, myEntityStore.$items);
  private loading = new StoreController(this, myEntityStore.$loading);
  private online = new StoreController(this, myEntityStore.$online);

  connectedCallback() {
    super.connectedCallback();
    void myEntityStore.refresh();  // Initial load
  }

  render() {
    return html`
      ${!this.online.value ? html`<offline-banner></offline-banner>` : ''}
      ${this.loading.value 
        ? html`<loading-spinner></loading-spinner>`
        : this.items.value.map(item => html`
            <entity-card
              .data=${item.data}
              .source=${item._meta.source}
              .syncStatus=${item._meta.syncStatus}
            ></entity-card>
          `)
      }
    `;
  }
}
```

### Store API

| Atom/Method | Description |
|-------------|-------------|
| `$items` | All items from all sources (merged, tagged) |
| `$loading` | Loading state |
| `$error` | Current error |
| `$sources` | Status of connected sources |
| `$pendingCount` | Pending offline mutations |
| `$online` | Online/offline status |
| `refresh()` | Fetch from all sources |
| `create(item, targetSource)` | Create in specific source |
| `update(id, changes)` | Update (auto-targets original source) |
| `delete(id)` | Delete (auto-targets original source) |
| `addSource(config)` | Add HTTP source at runtime |
| `removeSource(id)` | Remove source |
| `syncPending()` | Manually trigger offline sync |

### StoreItem Structure

```typescript
interface StoreItem<T> {
  data: T;                    // The actual entity
  _meta: {
    source: string;           // Source ID ('cloud', 'local', etc.)
    syncStatus: 'synced' | 'pending' | 'conflict' | 'error';
    lastSynced?: Date;
    errorMessage?: string;
  };
}
```

### Adding a New Source Type

To add a new backend (e.g., another cloud region):

```typescript
import { credentialsStore } from '../stores/credentials';

// At runtime
credentialsStore.addSource({
  id: 'cloud-eu',
  name: 'Cloud (EU)',
  baseUrl: 'https://eu.api.example.com/credentials',
  priority: 8,  // Lower than cloud-prod (10), higher than local (5)
});

// Items from this source will appear in $items with _meta.source === 'cloud-eu'
```

### Conflict Resolution

Default strategy: **highest priority source wins** when items have the same ID across sources.

Custom merge:
```typescript
const store = createMultiSourceStore<MyEntity>({
  name: 'my-entity',
  sources: [...],
  merge: (items) => {
    // items = all versions of same entity from different sources
    // Return the "winner"
    return items.sort((a, b) => 
      new Date(b.data.updatedAt).getTime() - new Date(a.data.updatedAt).getTime()
    )[0];
  },
});
```

## Components

### Source Settings (`source-settings.ts`)
Panel for managing credential sources at runtime:
- View connected sources and their status (connected/error/offline)
- Quick-add preset sources (Local, Cloud)
- Add custom HTTP sources with any endpoint
- Test source connectivity
- Remove sources

Access via Settings nav item or programmatically:
```typescript
const settings = document.getElementById('source-settings');
settings.visible = true;  // Show panel
```

## Generated Code
- `src/services/generated/` contains TypeSpec-generated API clients
- **DO NOT EDIT** generated files - they are excluded from strict TypeScript checks
- Wrap generated clients in store adapters for consistent interface
