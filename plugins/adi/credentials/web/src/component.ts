import { LitElement } from 'lit';
import { state } from 'lit/decorators.js';
import type { DataField } from './views/credential-form.js';
import {
  AdiCredentialsBusKey,
  type AdiCredentialsCreateEvent,
  type AdiCredentialsUpdateEvent,
  type CredentialAccessLog,
  type CredentialType,
  type CredentialWithCocoon,
  type CredentialWithDataAndCocoon,
  type VerifyResult,
} from './generated/bus-types.js';
import { renderCredentialList } from './views/credential-list.js';
import { renderCredentialDetail } from './views/credential-detail.js';
import { renderCredentialForm } from './views/credential-form.js';
import { cocoon } from './cocoon.js';

type ViewState =
  | { type: 'list' }
  | { type: 'detail'; credential: CredentialWithCocoon }
  | { type: 'create' }
  | { type: 'edit'; credential: CredentialWithCocoon };

export class AdiCredentialsElement extends LitElement {
  @state() private credentials: CredentialWithCocoon[] = [];
  @state() private revealedData: CredentialWithDataAndCocoon | null = null;
  @state() private verifyResult: VerifyResult | null = null;
  @state() private accessLogs: CredentialAccessLog[] = [];
  @state() private filter: CredentialType | undefined = undefined;
  @state() private searchQuery = '';
  @state() private viewState: ViewState = { type: 'list' };
  @state() private loading = false;
  @state() private submitting = false;
  @state() private confirmingDelete = false;
  @state() private error: string | null = null;
  @state() private dataFields: DataField[] = [{ key: '', value: '' }];
  @state() private selectedCocoonId = '';

  private unsubs: Array<() => void> = [];

  override createRenderRoot() { return this; }

  override connectedCallback(): void {
    super.connectedCallback();
    this.unsubs.push(
      this.bus.on(AdiCredentialsBusKey.ListChanged, ({ credentials }) => {
        this.credentials = credentials;
        this.loading = false;
      }, 'credentials-ui'),
      this.bus.on(AdiCredentialsBusKey.DetailChanged, ({ credential }) => {
        this.viewState = { type: 'detail', credential };
        this.loading = false;
      }, 'credentials-ui'),
      this.bus.on(AdiCredentialsBusKey.DataRevealed, ({ credential }) => {
        this.revealedData = credential;
      }, 'credentials-ui'),
      this.bus.on(AdiCredentialsBusKey.Verified, ({ result }) => {
        this.verifyResult = result;
      }, 'credentials-ui'),
      this.bus.on(AdiCredentialsBusKey.LogsChanged, ({ logs }) => {
        this.accessLogs = logs;
      }, 'credentials-ui'),
      this.bus.on(AdiCredentialsBusKey.Mutated, () => {
        this.submitting = false;
        this.selectedCocoonId = '';
        this.viewState = { type: 'list' };
        this.dataFields = [{ key: '', value: '' }];
        this.loadData();
      }, 'credentials-ui'),
      this.bus.on(AdiCredentialsBusKey.Deleted, ({ id }) => {
        this.credentials = this.credentials.filter(c => c.id !== id);
        this.viewState = { type: 'list' };
        this.confirmingDelete = false;
        this.submitting = false;
      }, 'credentials-ui'),
      this.bus.on(AdiCredentialsBusKey.Error, ({ message }) => {
        this.error = message;
        this.loading = false;
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

  private get selectedCredential(): CredentialWithCocoon | null {
    const v = this.viewState;
    return v.type === 'detail' || v.type === 'edit' ? v.credential : null;
  }

  private loadData(): void {
    this.loading = true;
    this.error = null;
    this.bus.emit(AdiCredentialsBusKey.List, {
      credential_type: this.filter,
    }, 'credentials-ui');
  }

  private selectCredential(cred: CredentialWithCocoon): void {
    this.revealedData = null;
    this.verifyResult = null;
    this.accessLogs = [];
    this.confirmingDelete = false;
    this.viewState = { type: 'detail', credential: cred };
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
    const cred = this.selectedCredential;
    if (!cred) return;
    this.bus.emit(AdiCredentialsBusKey.Reveal, { id: cred.id, cocoonId: cred.cocoonId }, 'credentials-ui');
  }

  private handleVerify(): void {
    const cred = this.selectedCredential;
    if (!cred) return;
    this.bus.emit(AdiCredentialsBusKey.Verify, { id: cred.id, cocoonId: cred.cocoonId }, 'credentials-ui');
  }

  private handleLoadLogs(): void {
    const cred = this.selectedCredential;
    if (!cred) return;
    this.bus.emit(AdiCredentialsBusKey.Logs, { id: cred.id, cocoonId: cred.cocoonId }, 'credentials-ui');
  }

  private handleDelete(): void {
    const cred = this.selectedCredential;
    if (!cred) return;
    if (!this.confirmingDelete) { this.confirmingDelete = true; return; }
    this.submitting = true;
    this.bus.emit(AdiCredentialsBusKey.Delete, { id: cred.id, cocoonId: cred.cocoonId }, 'credentials-ui');
  }

  private handleCreate(data: AdiCredentialsCreateEvent): void {
    this.submitting = true;
    this.error = null;
    this.bus.emit(AdiCredentialsBusKey.Create, data, 'credentials-ui');
  }

  private handleUpdate(data: AdiCredentialsUpdateEvent): void {
    this.submitting = true;
    this.error = null;
    this.bus.emit(AdiCredentialsBusKey.Update, data, 'credentials-ui');
  }

  private handleAddDataField(): void {
    this.dataFields = [...this.dataFields, { key: '', value: '' }];
  }

  private handleDataFieldChange(index: number, field: 'key' | 'value', val: string): void {
    this.dataFields = this.dataFields.map((f, i) =>
      i === index ? { ...f, [field]: val } : f,
    );
  }

  override render() {
    const view = this.viewState;

    if (view.type === 'detail') {
      return renderCredentialDetail({
        credential: view.credential,
        revealedData: this.revealedData,
        verifyResult: this.verifyResult,
        accessLogs: this.accessLogs,
        submitting: this.submitting,
        confirmingDelete: this.confirmingDelete,
        onBack: () => { this.viewState = { type: 'list' }; },
        onReveal: () => this.handleReveal(),
        onHide: () => { this.revealedData = null; },
        onVerify: () => this.handleVerify(),
        onLoadLogs: () => this.handleLoadLogs(),
        onDelete: () => this.handleDelete(),
        onCancelDelete: () => { this.confirmingDelete = false; },
        onEdit: () => { this.viewState = { type: 'edit', credential: view.credential }; },
      });
    }

    if (view.type === 'create' || view.type === 'edit') {
      const editing = view.type === 'edit' ? view.credential : null;
      return renderCredentialForm({
        cocoonInterface: cocoon,
        selectedCocoonId: this.selectedCocoonId,
        submitting: this.submitting,
        editing,
        dataFields: this.dataFields,
        onBack: () => {
          this.submitting = false;
          this.selectedCocoonId = '';
          this.dataFields = [{ key: '', value: '' }];
          this.viewState = view.type === 'edit'
            ? { type: 'detail', credential: view.credential }
            : { type: 'list' };
        },
        onCocoonSelected: (e) => { this.selectedCocoonId = e.detail.cocoonId; },
        onAddDataField: () => this.handleAddDataField(),
        onDataFieldChange: (i, f, v) => this.handleDataFieldChange(i, f, v),
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
      onNew: () => { this.submitting = false; this.selectedCocoonId = ''; this.dataFields = [{ key: '', value: '' }]; this.viewState = { type: 'create' }; },
    });
  }
}
