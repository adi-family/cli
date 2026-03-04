// Types generated from bus.tsp — see src/generated/bus/types.ts
export type { ActionPriority, ActionKindMode, ActionsPushEvent, ActionsDismissEvent, ActionsDismissedEvent, ActionsRegisterKindEvent } from './generated/bus';

export interface ActionCard {
  id: string;
  plugin: string;
  kind: string;
  data: Record<string, unknown>;
  priority: 'low' | 'normal' | 'urgent';
}

export type RenderFn = (data: Record<string, unknown>, actionId: string) => string;
export type KindMode = 'exclusive';
