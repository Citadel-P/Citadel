import { useCallback, useEffect, useMemo, useState, useSyncExternalStore, type ReactNode } from 'react';
import { focusManager, hashKey, useIsFetching, useQueryClient } from '@tanstack/react-query';
import { ReadAdmissionContext, ReadScopeContext, createReadAdmission } from '@/lib/read-admission';
import { requestErrorStatus } from '@/lib/request-error';
import { Badge } from '@/components/ui/badge';
import { OverviewTiles } from './overview';
import { supportedConfiguration } from './catalogue';
import { CaptureContext, Tile, ViewOnlyContext } from './tile';
import type { DashboardWidget } from '@/api/generated/api.types';
import { dashboardRequirements } from './requirements';
import { UnsupportedConfiguration } from './unsupported-configuration';

export type SavedTile = DashboardWidget;
const previewKey = (tile: DashboardWidget, tags: string[], range: number) =>
  JSON.stringify([tile.settings, tags, range]);
const isVisible = () => focusManager.isFocused();
export function DashboardGrid({
  configuration,
  paused = false,
  inactive = false,
  refresh = 0,
  onSettled,
  boardId,
}: {
  boardId?: string;
  configuration: unknown;
  paused?: boolean;
  inactive?: boolean;
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

  return (
    <ReadAdmissionContext.Provider value={admit}>
      <DataGrid
        boardId={boardId}
        tiles={tiles}
        tags={config?.tags ?? []}
        range={config?.rangeMinutes ?? 60}
        inactive={inactive}
        paused={paused}
        refresh={refresh}
        onSettled={onSettled}
      />
    </ReadAdmissionContext.Provider>
  );
}

function DataGrid({
  boardId,
  tiles,
  tags,
  range,
  paused,
  inactive,
  refresh,
  onSettled,
}: {
  boardId?: string;
  tiles?: DashboardWidget[];
  tags: string[];
  range: number;
  paused: boolean;
  inactive: boolean;
  refresh: number;
  onSettled?: () => void;
}) {
  const visible = useSyncExternalStore(focusManager.subscribe, isVisible);
  const client = useQueryClient();
  const plan = useMemo(() => dashboardRequirements(tiles ?? [], tags, range), [tiles, tags, range]);
  const demand = plan.scope;
  const globalAlerts = !tags.length && 'OpenAlerts' in plan.limits;
  // A scope lives as long as its data inputs, including while its observers are paused.
  const scope = useMemo(
    () => ({
      demand,
      snapshots: new Map<string, { key: string; value: ReactNode }>(),
      hashes: new Set<string>([
        hashKey(['getCurrentProfile', {}]),
        ...(boardId ? [hashKey(['getDashboard', { id: boardId }])] : []),
        ...(globalAlerts
          ? [hashKey(['listAlertEvents', { query: { Page: 1, PageSize: 5, UnresolvedOnly: true } }])]
          : []),
      ]),
    }),
    [boardId, demand, globalAlerts],
  );
  const { snapshots } = scope;
  const [observed, setObserved] = useState(refresh);
  const once = refresh !== observed;
  const [finished, setFinished] = useState(0);
  const settledTiles = useMemo(() => ({ generation: refresh, ids: new Set<string>() }), [refresh]);
  const capture = useCallback(
    (id: string, value: ReactNode, pending: boolean) => {
      const tile = tiles?.find((t) => t.id === id);
      if (tile) {
        const key = previewKey(tile, tags, range);
        snapshots.set(id, { key, value });
      }
      if (once) {
        if (pending) settledTiles.ids.delete(id);
        else settledTiles.ids.add(id);
        setFinished(settledTiles.ids.size);
      }
    },
    [snapshots, tiles, tags, range, once, settledTiles],
  );
  const relevant = useCallback((q: { queryHash: string }) => scope.hashes.has(q.queryHash), [scope]);
  const busy = useIsFetching({ predicate: relevant });
  const [, clear] = useState(0);
  useEffect(
    () =>
      client.getQueryCache().subscribe((e) => {
        if (!relevant(e.query)) return;
        if (
          e.type === 'removed' ||
          (e.type === 'updated' &&
            ((e.action.type === 'error' && [401, 403, 404].includes(requestErrorStatus(e.action.error) ?? 0)) ||
              e.action.type === 'invalidate'))
        ) {
          if (snapshots.size) {
            snapshots.clear();
            if (paused) clear((n) => n + 1);
          }
        }
      }),
    [client, paused, snapshots, relevant],
  );
  useEffect(() => {
    if (!once || busy || !visible || !tiles || finished !== tiles.length || settledTiles.ids.size !== tiles.length)
      return;
    // Query observers install their reads on mount. Finish after the commit,
    // including dependent sources that mounted for the first time on refresh.
    let current = true;
    queueMicrotask(() => {
      if (current && !client.isFetching({ predicate: relevant })) {
        setObserved(refresh);
        onSettled?.();
      }
    });
    return () => {
      current = false;
    };
  }, [once, busy, visible, tiles, finished, settledTiles, refresh, client, relevant, onSettled]);
  useEffect(() => {
    if (!once) onSettled?.();
  }, [once, onSettled]);
  if (!visible || inactive) return null;
  if (!tiles) return <UnsupportedConfiguration />;
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
        <div className="grid gap-3 md:grid-cols-12 md:auto-rows-[2.75rem]">
          {tiles.map((tile) => (
            <Tile key={tile.id} tile={tile}>
              {snapshots.get(tile.id)?.key === previewKey(tile, tags, range) ? (
                snapshots.get(tile.id)?.value
              ) : (
                <p>Paused · refresh to load this view.</p>
              )}
            </Tile>
          ))}
        </div>
      ) : (
        <ReadScopeContext.Provider value={scope.hashes}>
          <CaptureContext.Provider value={capture}>
            <OverviewTiles plan={plan} key={refresh} tiles={tiles} tags={tags} range={range} manual={once} />
          </CaptureContext.Provider>
        </ReadScopeContext.Provider>
      )}
    </ViewOnlyContext.Provider>
  );
}
