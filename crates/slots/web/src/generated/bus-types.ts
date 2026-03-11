/**
 * Auto-generated eventbus types from TypeSpec.
 * DO NOT EDIT.
 */

export interface SlotsDefineEvent {
  id: string;
  multiple?: boolean;
}

export interface SlotsPlaceEvent {
  slot: string;
  elementRef: unknown;
  priority?: number;
  pluginId: string;
}

export interface SlotsRemoveEvent {
  slot: string;
  elementRef: unknown;
}

export interface SlotsRemoveAllEvent {
  pluginId: string;
}

export interface SlotsChangedEvent {
  slot: string;
}

export enum SlotsBusKey {
  Define = 'slots:define',
  Place = 'slots:place',
  Remove = 'slots:remove',
  RemoveAll = 'slots:remove-all',
  Changed = 'slots:changed',
}
