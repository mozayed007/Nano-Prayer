import { writable } from 'svelte/store';
import type { VisualMode } from './types';

export const currentPrayer = writable<string | null>(null);
export const clockFormat = writable<'hour12' | 'hour24'>('hour12');
export const theme = writable<'system' | 'light' | 'dark'>('system');
export const visualMode = writable<VisualMode>('performance');
