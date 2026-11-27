import type { DashboardMetricSource } from '@/api/generated/api.types';
import { STATS_PRESETS } from './stats-range';

export function monitoringLink(
  path: string,
  kind: DashboardMetricSource['kind'],
  tab: 'logs' | 'stats',
  minutes?: number,
) {
  const parent = { Platform: tab, Deployment: 'container', Stack: 'services', SwarmService: 'runtime' }[kind];
  const query = new URLSearchParams();
  if (parent !== tab) query.set('view', tab);
  if (minutes !== undefined) query.set('minutes', String(minutes));
  return path + (query.size ? '?' + query : '') + '#' + parent;
}
export function initialRuntimeTab(fallback: string, logsAllowed: boolean) {
  const view = new URLSearchParams(window.location.search).get('view');
  return view === 'stats' || (view === 'logs' && logsAllowed) ? view : fallback;
}
export function initialHistoryMinutes() {
  const minutes = Number(new URLSearchParams(window.location.search).get('minutes'));
  return STATS_PRESETS.find((p) => p.minutes === minutes)?.minutes ?? 60;
}
