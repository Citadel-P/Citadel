import type { DashboardWidget, DashboardMetricSource, DashboardWorkloadKind } from '@/api/generated/api.types';
type TileOf<K extends DashboardWidget['settings']['kind']> = DashboardWidget & {
  settings: Extract<DashboardWidget['settings'], { kind: K }>;
};
export type MetricTile = TileOf<'Metrics'>;
export type WorkloadTile = TileOf<'Workloads'>;
function add<K, V>(groups: Map<K, V[]>, key: K, value: V) {
  const group = groups.get(key);
  if (group) group.push(value);
  else groups.set(key, [value]);
}
export function dashboardRequirements(tiles: DashboardWidget[], tags: string[], minutes: number) {
  const workloads = new Map<DashboardWorkloadKind, WorkloadTile[]>();
  const metrics = new Map<DashboardMetricSource['kind'], MetricTile[]>();
  const overview: DashboardWidget[] = [],
    maintenance: DashboardWidget[] = [],
    notes: TileOf<'Notes'>[] = [];
  const limits: Partial<Record<DashboardWidget['settings']['kind'], number>> = {};
  const demand = new Map<string, number>();
  for (const tile of tiles) {
    const s = tile.settings;
    const limit = 'limit' in s ? s.limit : 0;
    let source = '';
    switch (s.kind) {
      case 'Metrics':
        add(metrics, s.source.kind, tile as MetricTile);
        source = JSON.stringify([
          s.source.kind,
          s.source.id,
          s.source.kind === 'Container' ? s.source.platformId : null,
          minutes,
        ]);
        break;
      case 'Workloads':
        add(workloads, s.resourceKind, tile as WorkloadTile);
        source = s.resourceKind;
        break;
      case 'UpcomingSchedules':
      case 'Updates':
        maintenance.push(tile);
        break;
      case 'Notes':
        notes.push(tile as TileOf<'Notes'>);
        continue;
      default:
        overview.push(tile);
    }
    limits[s.kind] = Math.max(limits[s.kind] ?? 0, limit);
    const key = s.kind + source;
    demand.set(key, Math.max(demand.get(key) ?? 0, limit));
  }
  return {
    needsPlatforms: 'Platforms' in limits || 'ResourceSummary' in limits,
    workloads,
    metrics,
    overview,
    maintenance,
    notes,
    limits,
    scope: JSON.stringify([[...tags].sort(), [...demand].sort()]),
  };
}
export type DashboardRequirements = ReturnType<typeof dashboardRequirements>;
