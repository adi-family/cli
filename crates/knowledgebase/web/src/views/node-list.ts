import { html, nothing, type TemplateResult } from 'lit';
import type { SearchResult, NodeType } from '../types.js';
import { NODE_TYPE_COLORS, NODE_TYPE_LABELS, confidenceColor, truncate } from './shared.js';

interface NodeListProps {
  results: SearchResult[];
  searchQuery: string;
  filterType: NodeType | undefined;
  loading: boolean;
  error: string | null;
  onSelectNode(result: SearchResult): void;
  onFilterChange(type: NodeType | undefined): void;
  onSearch(query: string): void;
  onNewNode(): void;
}

const typeTabs = (current: NodeType | undefined, onChange: (t: NodeType | undefined) => void) => {
  const tabs: Array<{ label: string; value: NodeType | undefined }> = [
    { label: 'All', value: undefined },
    { label: 'Fact', value: 'fact' },
    { label: 'Decision', value: 'decision' },
    { label: 'Guide', value: 'guide' },
    { label: 'Error', value: 'error' },
    { label: 'Glossary', value: 'glossary' },
    { label: 'Context', value: 'context' },
    { label: 'Assumption', value: 'assumption' },
  ];

  return html`
    <div class="flex gap-1 mb-4 flex-wrap">
      ${tabs.map(
        (t) => html`
          <button
            class="px-3 py-1 rounded-full text-sm transition-colors ${
              current === t.value
                ? 'bg-purple-500/30 text-purple-200 font-medium'
                : 'bg-white/5 text-gray-400 hover:bg-white/10 hover:text-gray-200'
            }"
            @click=${() => onChange(t.value)}
          >
            ${t.label}
          </button>
        `
      )}
    </div>
  `;
};

const resultRow = (sr: SearchResult, onSelect: (sr: SearchResult) => void) => html`
  <button
    class="w-full text-left p-3 rounded-lg bg-white/5 hover:bg-white/10 transition-colors flex items-start gap-3 group"
    @click=${() => onSelect(sr)}
  >
    <span class="inline-flex px-2 py-0.5 rounded text-xs font-medium shrink-0 mt-0.5 ${NODE_TYPE_COLORS[sr.node.node_type]}">
      ${NODE_TYPE_LABELS[sr.node.node_type]}
    </span>
    <div class="flex-1 min-w-0">
      <div class="text-sm text-gray-200 group-hover:text-white truncate">${sr.node.title}</div>
      <div class="text-xs text-gray-500 mt-0.5 truncate">${truncate(sr.node.content, 120)}</div>
    </div>
    <div class="flex flex-col items-end shrink-0 mt-0.5 gap-0.5">
      <span class="text-xs text-gray-500">${(sr.score * 100).toFixed(0)}%</span>
      <span class="text-xs ${confidenceColor(sr.node.confidence[0])}">${sr.node.confidence[0].toFixed(1)}</span>
    </div>
  </button>
`;

export function renderNodeList(props: NodeListProps): TemplateResult {
  const filtered = props.filterType
    ? props.results.filter(sr => sr.node.node_type === props.filterType)
    : props.results;

  return html`
    <div class="space-y-3">
      <div class="flex items-center justify-between mb-2">
        <h2 class="text-lg font-semibold text-gray-200">Knowledge</h2>
        <button
          class="px-3 py-1.5 rounded-lg bg-purple-500/20 text-purple-200 hover:bg-purple-500/30 transition-colors text-sm font-medium"
          @click=${props.onNewNode}
        >
          + Add Knowledge
        </button>
      </div>

      ${typeTabs(props.filterType, props.onFilterChange)}

      <div class="relative mb-3">
        <input
          type="text"
          placeholder="Search knowledge..."
          .value=${props.searchQuery}
          @input=${(e: InputEvent) => props.onSearch((e.target as HTMLInputElement).value)}
          @keydown=${(e: KeyboardEvent) => {
            if (e.key === 'Enter') props.onSearch((e.target as HTMLInputElement).value);
          }}
          class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-purple-500/50"
        />
      </div>

      ${props.error
        ? html`<div class="p-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-300">${props.error}</div>`
        : props.loading
          ? html`<div class="text-center py-8 text-gray-500 text-sm">Searching...</div>`
          : filtered.length === 0
            ? html`<div class="text-center py-8 text-gray-500 text-sm">${props.searchQuery ? 'No results found' : 'Search for knowledge or add new entries'}</div>`
            : html`
                <div class="space-y-2">
                  ${filtered.map((sr) => resultRow(sr, props.onSelectNode))}
                </div>
              `}
    </div>
  `;
}
