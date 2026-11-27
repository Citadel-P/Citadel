import type { DashboardWidget, DashboardMetricSource } from '@/api/generated/api.types';
type TileOf<K extends DashboardWidget['settings']['kind']> = DashboardWidget & {
  settings: Extract<DashboardWidget['settings'], { kind: K }>;
};
export type FleetBoardTile = TileOf<'FleetSummary' | 'ResourceState' | 'NeedsAttention'>;
export type WatchTile = TileOf<'Watchlist'>;
export type MetricTile = TileOf<'Metrics'>;
function add<K, V>(groups: Map<K, V[]>, key: K, value: V) {
  const group = groups.get(key);
  if (group) group.push(value);
  else groups.set(key, [value]);
}
function sourceBatches(metrics: Map<DashboardMetricSource['kind'], MetricTile[]>, watches: WatchTile[]) {
  const pins = [
    ...[...metrics.values()].flat().map((t) => t.settings.source),
    ...watches.flatMap((w) => w.settings.pins),
  ];
  const groups = new Map<DashboardMetricSource['kind'], Set<string>>();
  for (const p of pins) {
    const set = groups.get(p.kind) ?? new Set<string>();
    set.add(p.id);
    groups.set(p.kind, set);
  }
  return [...groups]
    .sort(([a], [b]) => a.localeCompare(b))
    .flatMap(([kind, set]) => {
      const ids = [...set].sort();
      return Array.from({ length: Math.ceil(ids.length / 8) }, (_, i) => ({
        kind,
        ids: ids.slice(i * 8, i * 8 + 8),
      }));
    });
}

export function dashboardRequirements(tiles: DashboardWidget[]) {
  const metrics = new Map<DashboardMetricSource['kind'], MetricTile[]>();
  const fleet: FleetBoardTile[] = [],
    watches: WatchTile[] = [];
  const overview: DashboardWidget[] = [],
    maintenance: DashboardWidget[] = [],
    notes: TileOf<'Notes'>[] = [];
  const limits: Partial<Record<DashboardWidget['settings']['kind'], number>> = {};
  for (const tile of tiles) {
    const s = tile.settings;
    const limit = 'limit' in s ? s.limit : 0;
    switch (s.kind) {
      case 'FleetSummary':
      case 'ResourceState':
      case 'NeedsAttention':
        fleet.push(tile as FleetBoardTile);
        break;
      case 'Watchlist':
        watches.push(tile as WatchTile);
        break;
      case 'Metrics':
        add(metrics, s.source.kind, tile as MetricTile);
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
  }
  return {
    batches: sourceBatches(metrics, watches),
    fleet,
    watches,
    metrics,
    overview,
    maintenance,
    notes,
    limits,
  };
}
export type DashboardRequirements = ReturnType<typeof dashboardRequirements>;
