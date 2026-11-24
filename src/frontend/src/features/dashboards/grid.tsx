import { useCallback, useEffect, useMemo, useState, useSyncExternalStore, type ReactNode } from 'react';
import { focusManager, useIsFetching, useQueryClient } from '@tanstack/react-query';
import { ReadAdmissionContext, createReadAdmission } from '@/lib/read-admission';
import { Badge } from '@/components/ui/badge';
import { OverviewTiles } from './overview';
import { usePreviewStore, previewKey } from './preview';
import { supportedConfiguration } from './catalogue';
import { CaptureContext, Tile, ViewOnlyContext } from './tile';
import type { DashboardWidget } from '@/api/generated/api.types';
export type SavedTile = DashboardWidget;
const isVisible = () => focusManager.isFocused();
const sources = new Set([
  'getDashboard',
  'getCurrentProfile',
  'getApplicationInfo',
  'listDashboardSources',
  'getPlatformStats',
  'getContainerStats',
  'getStackStats',
  'getDeploymentStats',
  'getSwarmServiceStats',
  'getUpcomingSchedules',
  'listAlertEvents',
  'listRecentActivities',
  'listPlatforms',
  'resolveTagFilter',
]);

export function DashboardGrid({
  configuration,
  paused = false,
  refresh = 0,
  onSettled,
}: {
  configuration: unknown;
  paused?: boolean;
  refresh?: number;
  onSettled?: () => void;
}) {
  // Core admits up to two history reads per database pool.
  const [admit] = useState(() => createReadAdmission(2));
  const config = supportedConfiguration(configuration) ? configuration : undefined;
  const widgets = config?.widgets;
  const tiles = useMemo(
    () => widgets && [...widgets].sort((a, b) => a.layout.y - b.layout.y || a.layout.x - b.layout.x),
    [widgets],
  );
  const identity = JSON.stringify([config?.tags, config?.rangeMinutes, config?.widgets]);
  return (
    <ReadAdmissionContext.Provider value={admit}>
      <DataGrid
        key={identity}
        tiles={tiles}
        tags={config?.tags ?? []}
        range={config?.rangeMinutes ?? 60}
        paused={paused}
        refresh={refresh}
        onSettled={onSettled}
      />
    </ReadAdmissionContext.Provider>
  );
}

function DataGrid({
  tiles,
  tags,
  range,
  paused,
  refresh,
  onSettled,
}: {
  tiles?: DashboardWidget[];
  tags: string[];
  range: number;
  paused: boolean;
  refresh: number;
  onSettled?: () => void;
}) {
  const visible = useSyncExternalStore(focusManager.subscribe, isVisible);
  const client = useQueryClient();
  const [snapshots] = useState(() => new Map<string, ReactNode>());
  const previews = usePreviewStore();
  const [observed, setObserved] = useState(refresh);
  const once = refresh !== observed;
  const [finished, setFinished] = useState(0);
  const settledTiles = useMemo(() => ({ generation: refresh, ids: new Set<string>() }), [refresh]);
  const capture = useCallback(
    (id: string, value: ReactNode, pending: boolean) => {
      snapshots.set(id, value);
      const tile = tiles?.find((t) => t.id === id);
      if (tile) previews?.capture(id, previewKey(tile, tags, range), value);
      if (once) {
        if (pending) settledTiles.ids.delete(id);
        else settledTiles.ids.add(id);
        setFinished(settledTiles.ids.size);
      }
    },
    [snapshots, tiles, tags, range, previews, once, settledTiles],
  );
  const busy = useIsFetching({ predicate: (q) => sources.has(String(q.queryKey[0])) });
  const [, clear] = useState(0);
  useEffect(
    () =>
      client.getQueryCache().subscribe((e) => {
        if (!sources.has(String(e.query.queryKey[0]))) return;
        if (
          e.type === 'removed' ||
          (e.type === 'updated' && (e.action.type === 'error' || e.action.type === 'invalidate'))
        ) {
          previews?.clear();
          if (snapshots.size) {
            snapshots.clear();
            if (paused) clear((n) => n + 1);
          }
        }
      }),
    [client, paused, snapshots, previews],
  );
  useEffect(() => {
    if (!once || busy || !visible || !tiles || finished !== tiles.length || settledTiles.ids.size !== tiles.length)
      return;
    // Query observers install their reads on mount. Finish after the commit,
    // including dependent sources that mounted for the first time on refresh.
    let current = true;
    queueMicrotask(() => {
      if (current && !client.isFetching({ predicate: (q) => sources.has(String(q.queryKey[0])) })) {
        setObserved(refresh);
        onSettled?.();
      }
    });
    return () => {
      current = false;
    };
  }, [once, busy, visible, tiles, finished, settledTiles, refresh, client, onSettled]);
  useEffect(() => {
    if (!once) onSettled?.();
  }, [once, onSettled]);
  if (!visible) return null;
  if (!tiles) return <p>This dashboard uses an unsupported configuration.</p>;
  if (!tiles.length) return <p className="text-muted-foreground">This dashboard has no widgets yet.</p>;
  return (
    <ViewOnlyContext.Provider value={paused}>
      {tags.length > 0 && (
        <ul aria-label="Dashboard tag filter" className="flex flex-wrap gap-2">
          {tags.map((t) => (
            <li key={t}>
              <Badge variant="outline">{t}</Badge>
            </li>
          ))}
        </ul>
      )}
      {paused && !once ? (
        <div className="grid gap-4 md:grid-cols-12 md:auto-rows-[3.5rem]">
          {tiles.map((tile) => (
            <Tile key={tile.id} tile={tile}>
              {snapshots.get(tile.id) ?? <p>Paused · refresh to load this view.</p>}
            </Tile>
          ))}
        </div>
      ) : (
        <CaptureContext.Provider value={capture}>
          <OverviewTiles key={refresh} tiles={tiles} tags={tags} range={range} manual={once} />
        </CaptureContext.Provider>
      )}
    </ViewOnlyContext.Provider>
  );
}
