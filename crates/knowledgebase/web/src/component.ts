import { LitElement } from 'lit';
import { state } from 'lit/decorators.js';
import type { Node, Edge, SearchResult, NodeType, Connection } from './types.js';
import { renderNodeList } from './views/node-list.js';
import { renderNodeDetail } from './views/node-detail.js';
import { renderNodeForm } from './views/node-form.js';

type View = 'list' | 'detail' | 'add';

export class AdiKnowledgebaseElement extends LitElement {
  @state() private results: SearchResult[] = [];
  @state() private selectedNode: Node | null = null;
  @state() private selectedEdges: Edge[] = [];
  @state() private filterType: NodeType | undefined = undefined;
  @state() private searchQuery = '';
  @state() private view: View = 'list';
  @state() private loading = false;
  @state() private submitting = false;
  @state() private confirmingDelete = false;
  @state() private error: string | null = null;

  private unsubs: Array<() => void> = [];

  override createRenderRoot() { return this; }

  private get bus() { return window.sdk.bus; }

  override connectedCallback(): void {
    super.connectedCallback();
    this.unsubs.push(
      this.bus.on('kb:results-changed', ({ results }) => {
        this.results = results;
        this.loading = false;
      }, 'kb-ui'),
      this.bus.on('kb:node-changed', ({ node, edges }) => {
        this.selectedNode = node;
        this.selectedEdges = edges;
        this.submitting = false;
      }, 'kb-ui'),
      this.bus.on('kb:node-deleted', ({ id }) => {
        this.results = this.results.filter(sr => sr.node.id !== id);
        this.view = 'list';
        this.selectedNode = null;
        this.confirmingDelete = false;
        this.submitting = false;
      }, 'kb-ui'),
    );
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    this.unsubs.forEach(fn => fn());
    this.unsubs = [];
  }

  private doSearch(): void {
    if (!this.searchQuery.trim()) {
      this.results = [];
      return;
    }
    this.loading = true;
    this.error = null;
    this.bus.emit('kb:query', { q: this.searchQuery }, 'kb-ui');
  }

  private handleSearch(query: string): void {
    this.searchQuery = query;
    this.doSearch();
  }

  private selectNode(sr: SearchResult): void {
    this.selectedNode = sr.node;
    this.selectedEdges = sr.edges;
    this.view = 'detail';
    this.confirmingDelete = false;
  }

  private handleApprove(): void {
    if (!this.selectedNode) return;
    this.submitting = true;
    this.bus.emit('kb:approve', { id: this.selectedNode.id, cocoonId: this.selectedNode.cocoonId }, 'kb-ui');
  }

  private handleDelete(): void {
    if (!this.selectedNode) return;
    if (!this.confirmingDelete) { this.confirmingDelete = true; return; }
    this.submitting = true;
    this.bus.emit('kb:delete', { id: this.selectedNode.id, cocoonId: this.selectedNode.cocoonId }, 'kb-ui');
  }

  private handleCreate(data: { user_said: string; derived_knowledge: string; node_type?: string; cocoonId: string }): void {
    this.submitting = true;
    this.bus.emit('kb:add', data, 'kb-ui');
    this.view = 'list';
    if (this.searchQuery.trim()) {
      this.doSearch();
    }
  }

  override render() {
    const allConnections: Connection[] = [...window.sdk.getConnections().values()];

    if (this.view === 'detail' && this.selectedNode) {
      return renderNodeDetail({
        node: this.selectedNode,
        edges: this.selectedEdges,
        submitting: this.submitting,
        confirmingDelete: this.confirmingDelete,
        onBack: () => { this.view = 'list'; this.selectedNode = null; this.confirmingDelete = false; },
        onApprove: () => this.handleApprove(),
        onDelete: () => this.handleDelete(),
        onCancelDelete: () => { this.confirmingDelete = false; },
      });
    }

    if (this.view === 'add') {
      return renderNodeForm({
        connections: allConnections,
        submitting: this.submitting,
        onBack: () => { this.view = 'list'; },
        onCreate: (data) => this.handleCreate(data),
      });
    }

    return renderNodeList({
      results: this.results,
      searchQuery: this.searchQuery,
      filterType: this.filterType,
      loading: this.loading,
      error: this.error,
      onSelectNode: (sr) => this.selectNode(sr),
      onFilterChange: (type) => { this.filterType = type; },
      onSearch: (query) => this.handleSearch(query),
      onNewNode: () => { this.view = 'add'; },
    });
  }
}
