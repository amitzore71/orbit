import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { AppCommand, Reply, ReportQuery } from './protocol';
export interface Adapter {
  invoke<T>(name: string, args?: Record<string, unknown>): Promise<T>;
  listen<T>(name: string, handler: (value: T) => void): Promise<() => void>;
}
declare global {interface Window {__ORBIT_TEST__?: Adapter}}
export function adapter(): Adapter {
  if (process.env.NODE_ENV !== 'production' && typeof window !== 'undefined' && window.__ORBIT_TEST__) return window.__ORBIT_TEST__;
  return {invoke, listen: async <T,>(name: string, handler: (value: T) => void) => listen<T>(name, e => handler(e.payload))};
}
export function command(command: AppCommand) {return adapter().invoke<Reply>('orbit_command', {command});}
export function windowAction(action: string) {return adapter().invoke<void>('window_command', {action});}
export function chooseFile(kind: number, query?: ReportQuery) {return adapter().invoke<string | null>('file_operation', {kind, query});}
export function normalizeError(value: unknown): {message: string; accepted: boolean} {
  if (value && typeof value === 'object' && 'message' in value) return {message: String(value.message), accepted: 'accepted' in value && Boolean(value.accepted)};
  return {message: String(value || 'Could not complete this action.'), accepted: false};
}
