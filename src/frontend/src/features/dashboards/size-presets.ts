import type { WidgetKind } from './catalogue';

type Size = readonly [width: number, height: number, rows?: number];
const eventSizes = [
  [6, 5, 2],
  [6, 7, 4],
  [8, 10, 6],
] as const;
export const sizePresets: Record<WidgetKind, readonly Size[]> = {
  FleetSummary: [[12, 2]],
  NeedsAttention: [
    [6, 5, 3],
    [8, 7, 5],
    [12, 11, 10],
  ],
  ResourceState: [
    [4, 3],
    [5, 4],
    [6, 5],
  ],
  Metrics: [
    [4, 4],
    [6, 5],
    [8, 7],
  ],
  Watchlist: [
    [6, 5],
    [6, 7],
    [8, 10],
  ],
  RecentActivity: eventSizes,
  OpenAlerts: eventSizes,
  Backups: eventSizes,
  UpcomingSchedules: [
    [6, 4, 3],
    [6, 5, 4],
    [8, 7, 6],
  ],
  Updates: [
    [6, 4],
    [6, 5],
  ],
  Notes: [
    [4, 3],
    [6, 4],
  ],
};
