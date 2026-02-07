/**
 * Indexer HTTP API
 *
 * Code indexing, symbol search, file analysis, and dead code detection.
 */

import "@typespec/http";

using TypeSpec.Http;

namespace Indexer;

// -- Enums --

enum SymbolKind {
  function: "function",
  method: "method",
  class: "class",
  struct: "struct",
  enum: "enum",
  interface: "interface",
  trait: "trait",
  module: "module",
  constant: "constant",
  variable: "variable",
  type: "type",
  property: "property",
  field: "field",
  constructor: "constructor",
  destructor: "destructor",
  operator: "operator",
  macro: "macro",
  namespace: "namespace",
  package: "package",
  unknown: "unknown",
}

enum Visibility {
  public: "public",
  publicCrate: "public_crate",
  publicSuper: "public_super",
  protected: "protected",
  private: "private",
  internal: "internal",
  unknown: "unknown",
}

// -- Models --

model Location {
  startLine: uint32;
  startCol: uint32;
  endLine: uint32;
  endCol: uint32;
  startByte: uint32;
  endByte: uint32;
}

model Symbol {
  id: int64;
  name: string;
  kind: SymbolKind;
  fileId: int64;
  filePath: string;
  location: Location;
  parentId?: int64;
  signature?: string;
  description?: string;
  docComment?: string;
  visibility: Visibility;
  isEntryPoint: boolean;
}

model SearchResult {
  symbol: Symbol;
  score: float32;
  context?: string;
}

model File {
  id: int64;
  path: string;
  language: string;
  hash: string;
  size: uint64;
  description?: string;
}

model FileInfo {
  file: File;
  symbols: Symbol[];
}

model Status {
  indexedFiles: uint64;
  indexedSymbols: uint64;
  embeddingDimensions: uint32;
  embeddingModel: string;
  lastIndexed?: string;
  storageSizeBytes: uint64;
}

model IndexProgress {
  filesProcessed: uint64;
  filesTotal: uint64;
  symbolsIndexed: uint64;
  errors: string[];
}

model SymbolNode {
  id: int64;
  name: string;
  kind: SymbolKind;
  children: SymbolNode[];
}

model FileNode {
  path: string;
  language: string;
  symbols: SymbolNode[];
}

model Tree {
  files: FileNode[];
}

model ReportSummary {
  totalSymbols: int32;
  reachable: int32;
  unreachable: int32;
  byKind: Record<int32>;
}

model DeadCodeReport {
  deadSymbols: int64[];
  entryPoints: Symbol[];
  summary: ReportSummary;
}

model ReachabilityResponse {
  symbolId: int64;
  symbolName: string;
  isReachable: boolean;
  status: string;
}

model SearchQueryParams {
  @query q: string;
  @query limit?: int32;
}

model DeadCodeQueryParams {
  @query mode?: string;
  @query excludeTests?: boolean;
  @query excludeTraits?: boolean;
  @query excludeFfi?: boolean;
}

// -- Interfaces --

interface IndexerService {
  @get
  @route("/status")
  getStatus(): {
    @statusCode statusCode: 200;
    @body body: Status;
  };

  @post
  @route("/index")
  indexProject(): {
    @statusCode statusCode: 200;
    @body body: IndexProgress;
  };

  @get
  @route("/search")
  search(...SearchQueryParams): {
    @statusCode statusCode: 200;
    @body body: SearchResult[];
  };

  @get
  @route("/symbols")
  searchSymbols(...SearchQueryParams): {
    @statusCode statusCode: 200;
    @body body: SearchResult[];
  };

  @get
  @route("/symbols/{id}")
  getSymbol(@path id: int64): {
    @statusCode statusCode: 200;
    @body body: Symbol;
  };

  @get
  @route("/symbols/{id}/reachability")
  getSymbolReachability(@path id: int64): {
    @statusCode statusCode: 200;
    @body body: ReachabilityResponse;
  };

  @get
  @route("/files")
  searchFiles(...SearchQueryParams): {
    @statusCode statusCode: 200;
    @body body: File[];
  };

  @get
  @route("/files/{path}")
  getFile(@path path: string): {
    @statusCode statusCode: 200;
    @body body: FileInfo;
  };

  @get
  @route("/tree")
  getTree(): {
    @statusCode statusCode: 200;
    @body body: Tree;
  };

  @get
  @route("/dead-code")
  findDeadCode(...DeadCodeQueryParams): {
    @statusCode statusCode: 200;
    @body body: DeadCodeReport;
  };
}
