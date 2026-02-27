import { LitElement } from 'lit';
import { state } from 'lit/decorators.js';
import type { Node, Edge, SearchResult, NodeType, Connection } from './types.js';
import { renderNodeList } from './views/node-list.js';
import { renderNodeDetail } from './views/node-detail.js';
import { renderNodeForm } from './views/node-form.js';

declare global {
  interface Window {
    sdk: {
      bus: import('@adi-family/sdk-plugin').EventBus;
      getConnections(): Map<string, Connection>;
    };
  }
}

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

  override createRenderRoot() { return this; }

  private get bus() { return window.sdk.bus; }

  private async doSearch(): Promise<void> {
    if (!this.searchQuery.trim()) {
      this.results = [];
      return;
    }
    this.loading = true;
    this.error = null;
    try {
      const result = await this.bus.send('kb:query', { q: this.searchQuery }, 'kb-ui').wait();
      this.results = result.results;
    } catch (err) {
      this.error = err instanceof Error ? err.message : 'Search failed';
    } finally {
      this.loading = false;
    }
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

  private async handleApprove(): Promise<void> {
    if (!this.selectedNode) return;
    this.submitting = true;
    try {
      await this.bus.send('kb:approve', { id: this.selectedNode.id, cocoonId: this.selectedNode.cocoonId }, 'kb-ui').wait();
      this.selectedNode = { ...this.selectedNode, confidence: { 0: 1.0 } };
    } catch (err) {
      this.error = err instanceof Error ? err.message : 'Approve failed';
    } finally {
      this.submitting = false;
    }
  }

  private async handleDelete(): Promise<void> {
    if (!this.selectedNode) return;
    if (!this.confirmingDelete) { this.confirmingDelete = true; return; }
    this.submitting = true;
    try {
      await this.bus.send('kb:delete', { id: this.selectedNode.id, cocoonId: this.selectedNode.cocoonId }, 'kb-ui').wait();
      this.results = this.results.filter(sr => sr.node.id !== this.selectedNode!.id);
      this.view = 'list';
      this.selectedNode = null;
      this.confirmingDelete = false;
    } catch (err) {
      this.error = err instanceof Error ? err.message : 'Delete failed';
      this.confirmingDelete = false;
    } finally {
      this.submitting = false;
    }
  }

  private async handleCreate(data: { user_said: string; derived_knowledge: string; node_type?: string; cocoonId: string }): Promise<void> {
    this.submitting = true;
    try {
      await this.bus.send('kb:add', data, 'kb-ui').wait();
      this.view = 'list';
      if (this.searchQuery.trim()) {
        this.doSearch();
      }
    } catch (err) {
      this.error = err instanceof Error ? err.message : 'Failed to add knowledge';
    } finally {
      this.submitting = false;
    }
  }

  override render() {
    const connections: Connection[] = [...window.sdk.getConnections().values()];

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
        connections,
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
