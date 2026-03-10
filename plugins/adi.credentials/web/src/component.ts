import { LitElement } from 'lit';
import { state } from 'lit/decorators.js';
import type { CocoonOption } from './views/credential-form.js';
import type {
  Credential,
  CredentialAccessLog,
  CredentialType,
  CredentialWithData,
  VerifyResult,
} from './types.js';
import { renderCredentialList } from './views/credential-list.js';
import { renderCredentialDetail } from './views/credential-detail.js';
import { renderCredentialForm } from './views/credential-form.js';
import { cocoon } from './cocoon.js';

type View = 'list' | 'detail' | 'create' | 'edit';

export class AdiCredentialsElement extends LitElement {
  @state() private credentials: Credential[] = [];
  @state() private selected: Credential | null = null;
  @state() private revealedData: CredentialWithData | null = null;
  @state() private verifyResult: VerifyResult | null = null;
  @state() private accessLogs: CredentialAccessLog[] = [];
  @state() private filter: CredentialType | undefined = undefined;
  @state() private searchQuery = '';
  @state() private view: View = 'list';
  @state() private loading = false;
  @state() private submitting = false;
  @state() private confirmingDelete = false;
  @state() private error: string | null = null;

  private unsubs: Array<() => void> = [];

  override createRenderRoot() { return this; }

  override connectedCallback(): void {
    super.connectedCallback();
    this.unsubs.push(
      this.bus.on('credentials:list-changed', ({ credentials }) => {
        this.credentials = credentials;
        this.loading = false;
      }, 'credentials-ui'),
      this.bus.on('credentials:detail-changed', ({ credential }) => {
        this.selected = credential;
        this.loading = false;
      }, 'credentials-ui'),
      this.bus.on('credentials:data-revealed', ({ credential }) => {
        this.revealedData = credential;
      }, 'credentials-ui'),
      this.bus.on('credentials:verified', ({ result }) => {
        this.verifyResult = result;
      }, 'credentials-ui'),
      this.bus.on('credentials:logs-changed', ({ logs }) => {
        this.accessLogs = logs;
      }, 'credentials-ui'),
      this.bus.on('credentials:mutated', () => {
        this.submitting = false;
        this.view = 'list';
        this.loadData();
      }, 'credentials-ui'),
      this.bus.on('credentials:deleted', ({ id }) => {
        this.credentials = this.credentials.filter(c => c.id !== id);
        this.view = 'list';
        this.selected = null;
        this.confirmingDelete = false;
        this.submitting = false;
      }, 'credentials-ui'),
    );
    this.loadData();
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    this.unsubs.forEach(fn => fn());
    this.unsubs = [];
  }

  private get bus() { return cocoon.bus; }

  private loadData(): void {
    this.loading = true;
    this.error = null;
    this.bus.emit('credentials:list', {
      credential_type: this.filter,
    }, 'credentials-ui');
  }

  private selectCredential(cred: Credential): void {
    this.selected = cred;
    this.revealedData = null;
    this.verifyResult = null;
    this.accessLogs = [];
    this.confirmingDelete = false;
    this.view = 'detail';
  }

  private handleFilterChange(type: CredentialType | undefined): void {
    this.filter = type;
    this.loadData();
  }

  private handleSearch(query: string): void {
    this.searchQuery = query;
    this.requestUpdate();
  }

  private handleReveal(): void {
    if (!this.selected) return;
    this.bus.emit('credentials:reveal', { id: this.selected.id, cocoonId: this.selected.cocoonId }, 'credentials-ui');
  }

  private handleHide(): void {
    this.revealedData = null;
  }

  private handleVerify(): void {
    if (!this.selected) return;
    this.bus.emit('credentials:verify', { id: this.selected.id, cocoonId: this.selected.cocoonId }, 'credentials-ui');
  }

  private handleLoadLogs(): void {
    if (!this.selected) return;
    this.bus.emit('credentials:logs', { id: this.selected.id, cocoonId: this.selected.cocoonId }, 'credentials-ui');
  }

  private handleDelete(): void {
    if (!this.selected) return;
    if (!this.confirmingDelete) { this.confirmingDelete = true; return; }
    this.submitting = true;
    this.bus.emit('credentials:delete', { id: this.selected.id, cocoonId: this.selected.cocoonId }, 'credentials-ui');
  }

  private handleCreate(data: {
    cocoonId: string;
    name: string;
    credential_type: CredentialType;
    data: Record<string, unknown>;
    description?: string;
    provider?: string;
    expires_at?: string;
  }): void {
    this.submitting = true;
    this.bus.emit('credentials:create', data, 'credentials-ui');
  }

  private handleUpdate(data: {
    cocoonId: string;
    id: string;
    name?: string;
    description?: string;
    data?: Record<string, unknown>;
    provider?: string;
    expires_at?: string;
  }): void {
    this.submitting = true;
    this.bus.emit('credentials:update', data, 'credentials-ui');
  }

  override render() {
    const credConns = new Set(cocoon.connectionsWithPlugin('adi.credentials').map(c => c.id));
    const cocoons: CocoonOption[] = cocoon.cocoonDevices().map(d => ({
      id: d.device_id,
      installed: credConns.has(d.device_id),
    }));

    if (this.view === 'detail' && this.selected) {
      return renderCredentialDetail({
        credential: this.selected,
        revealedData: this.revealedData,
        verifyResult: this.verifyResult,
        accessLogs: this.accessLogs,
        submitting: this.submitting,
        confirmingDelete: this.confirmingDelete,
        onBack: () => { this.view = 'list'; this.selected = null; },
        onReveal: () => this.handleReveal(),
        onHide: () => this.handleHide(),
        onVerify: () => this.handleVerify(),
        onLoadLogs: () => this.handleLoadLogs(),
        onDelete: () => this.handleDelete(),
        onCancelDelete: () => { this.confirmingDelete = false; },
        onEdit: () => { this.view = 'edit'; },
      });
    }

    if (this.view === 'create' || this.view === 'edit') {
      return renderCredentialForm({
        cocoons,
        submitting: this.submitting,
        editing: this.view === 'edit' ? this.selected : null,
        onBack: () => { this.view = this.selected ? 'detail' : 'list'; this.submitting = false; },
        onCreate: (data) => this.handleCreate(data),
        onUpdate: (data) => this.handleUpdate(data),
      });
    }

    return renderCredentialList({
      credentials: this.credentials,
      filter: this.filter,
      searchQuery: this.searchQuery,
      loading: this.loading,
      error: this.error,
      onSelect: (c) => this.selectCredential(c),
      onFilterChange: (t) => this.handleFilterChange(t),
      onSearch: (q) => this.handleSearch(q),
      onNew: () => { this.view = 'create'; this.submitting = false; },
    });
  }
}
