import type { NodeType } from '../types.js';

export const NODE_TYPE_COLORS: Record<NodeType, string> = {
  decision: 'bg-purple-500/20 text-purple-300',
  fact: 'bg-blue-500/20 text-blue-300',
  error: 'bg-red-500/20 text-red-300',
  guide: 'bg-green-500/20 text-green-300',
  glossary: 'bg-yellow-500/20 text-yellow-300',
  context: 'bg-cyan-500/20 text-cyan-300',
  assumption: 'bg-orange-500/20 text-orange-300',
};

export const NODE_TYPE_LABELS: Record<NodeType, string> = {
  decision: 'Decision',
  fact: 'Fact',
  error: 'Error',
  guide: 'Guide',
  glossary: 'Glossary',
  context: 'Context',
  assumption: 'Assumption',
};

export function confidenceLabel(value: number): string {
  if (value >= 1.0) return 'Approved';
  if (value >= 0.8) return 'Strong';
  if (value >= 0.5) return 'Medium';
  return 'Weak';
}

export function confidenceColor(value: number): string {
  if (value >= 1.0) return 'text-green-400';
  if (value >= 0.8) return 'text-blue-400';
  if (value >= 0.5) return 'text-yellow-400';
  return 'text-red-400';
}

export function truncate(text: string, max: number): string {
  return text.length > max ? text.slice(0, max) + '...' : text;
}

export function formatDate(iso: string): string {
  return new Date(iso).toLocaleString();
}
