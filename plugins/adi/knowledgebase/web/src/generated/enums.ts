/**
 * Auto-generated enums from TypeSpec.
 * DO NOT EDIT.
 */


export enum NodeType {
  Decision = "decision",
  Fact = "fact",
  Error = "error",
  Guide = "guide",
  Glossary = "glossary",
  Context = "context",
  Assumption = "assumption",
}

export enum EdgeType {
  Supersedes = "supersedes",
  Contradicts = "contradicts",
  Requires = "requires",
  RelatedTo = "related_to",
  DerivedFrom = "derived_from",
  Answers = "answers",
}

export enum ApprovalStatus {
  Pending = "pending",
  Approved = "approved",
  Rejected = "rejected",
}

export enum AuditAction {
  Create = "create",
  Update = "update",
  Delete = "delete",
  Approve = "approve",
  Reject = "reject",
}
