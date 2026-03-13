import { html, nothing, type TemplateResult } from 'lit';
import type { Node, Edge } from '../types.js';
import { NODE_TYPE_COLORS, NODE_TYPE_LABELS, confidenceLabel, confidenceColor, formatDate } from './shared.js';

interface NodeDetailProps {
  node: Node;
  edges: Edge[];
  submitting: boolean;
  confirmingDelete: boolean;
  onBack(): void;
  onApprove(): void;
  onDelete(): void;
  onCancelDelete(): void;
}

const edgeRow = (edge: Edge) => html`
  <div class="px-3 py-2 rounded-lg bg-white/5 flex items-center gap-2 text-sm">
    <span class="text-xs px-1.5 py-0.5 rounded bg-white/10 text-gray-400">${edge.edge_type}</span>
    <span class="text-gray-500 text-xs">${edge.from_id.slice(0, 8)} &rarr; ${edge.to_id.slice(0, 8)}</span>
    <span class="text-xs text-gray-600 ml-auto">w: ${edge.weight.toFixed(2)}</span>
  </div>
`;

export function renderNodeDetail(props: NodeDetailProps): TemplateResult {
  const { node, edges, submitting, confirmingDelete, onBack, onApprove, onDelete, onCancelDelete } = props;
  const conf = node.confidence[0];

  return html`
    <div class="space-y-4">
      <button class="text-sm text-gray-400 hover:text-gray-200 transition-colors" @click=${onBack}>
        &larr; Back to search
      </button>

      <div class="bg-white/5 rounded-xl p-4 space-y-4">
        <div class="flex items-start gap-2">
          <span class="inline-flex px-2 py-0.5 rounded text-xs font-medium ${NODE_TYPE_COLORS[node.node_type]}">
            ${NODE_TYPE_LABELS[node.node_type]}
          </span>
          <span class="text-xs ${confidenceColor(conf)}">${confidenceLabel(conf)} (${conf.toFixed(2)})</span>
        </div>

        <h2 class="text-lg font-semibold text-gray-100">${node.title}</h2>

        <p class="text-sm text-gray-400 whitespace-pre-wrap">${node.content}</p>

        ${node.source.User
          ? html`
              <div class="p-3 rounded-lg bg-white/5 border border-white/10">
                <div class="text-xs text-gray-500 uppercase tracking-wider mb-1">User said</div>
                <div class="text-sm text-gray-300 italic">"${node.source.User.statement}"</div>
              </div>
            `
          : nothing}

        <div class="flex gap-4 text-xs text-gray-500">
          <span>Created: ${formatDate(node.created_at)}</span>
          <span>Updated: ${formatDate(node.updated_at)}</span>
        </div>

        ${edges.length > 0
          ? html`
              <div>
                <h4 class="text-xs font-medium text-gray-400 uppercase tracking-wider mb-2">Edges (${edges.length})</h4>
                <div class="space-y-1">
                  ${edges.map(edgeRow)}
                </div>
              </div>
            `
          : nothing}

        <div class="pt-3 border-t border-white/10 flex gap-2">
          ${conf < 1.0
            ? html`
                <button
                  class="px-3 py-1 rounded text-sm bg-green-500/20 text-green-300 hover:bg-green-500/30 transition-colors"
                  ?disabled=${submitting}
                  @click=${onApprove}
                >
                  Approve
                </button>
              `
            : nothing}

          ${confirmingDelete
            ? html`
                <div class="flex items-center gap-2">
                  <span class="text-sm text-red-400">Delete this node?</span>
                  <button
                    class="px-3 py-1 rounded text-sm bg-red-500/20 text-red-300 hover:bg-red-500/30 transition-colors"
                    ?disabled=${submitting}
                    @click=${onDelete}
                  >
                    Confirm
                  </button>
                  <button
                    class="px-3 py-1 rounded text-sm bg-white/5 text-gray-400 hover:bg-white/10 transition-colors"
                    @click=${onCancelDelete}
                  >
                    Cancel
                  </button>
                </div>
              `
            : html`
                <button
                  class="px-3 py-1 rounded text-sm bg-red-500/10 text-red-400 hover:bg-red-500/20 transition-colors"
                  ?disabled=${submitting}
                  @click=${onDelete}
                >
                  Delete
                </button>
              `}
        </div>
      </div>
    </div>
  `;
}
