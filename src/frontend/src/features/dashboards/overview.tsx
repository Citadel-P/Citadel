import { useMemo, useState, type ReactNode } from 'react';
import { Link } from 'react-router';
import {
  ActivityEventType,
  PlatformStatus,
  type PlatformView,
  type RecentActivityView,
} from '@/api/generated/api.types';
import { StateBadge } from '@/components/custom/state-badge';
import { Timestamp } from '@/components/custom/timestamp';
import { useAlertPage } from '@/features/alerters/alert-events/hooks/useAlertEventsList';
import { useAppContext } from '@/lib/context/app-context';
import { useRealtimeContext } from '@/lib/context/realtime-context';
import { useRead } from '@/lib/hooks';
import { formatActivityEvent } from '@/lib/utils';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import type { DateTimeFormatter } from '@/lib/date-time';
import { usePlatformCollection } from '@/features/platforms/hooks/usePlatformsGroup';
import { useResolvedTagFilter } from '@/features/tags/components';
import { Tile, OrderedTiles } from './tile';
import { SupplementalTiles } from './widget-data';
import { ActivityActions, CopyResourceLink } from './row-actions';
import { monitoringLink } from '@/lib/monitoring-link';
import { AlertRow } from './alert-row';
import type { SavedTile } from './grid';

function recentRows(rows: RecentActivityView[], limit: number, backups: boolean, format: DateTimeFormatter) {
  return rows.slice(0, limit).map((row) => (
    <li key={row.id} className="flex flex-wrap items-center justify-between gap-2 py-1.5">
      <Link className="table-link min-w-0 break-words" to={'/activities/' + row.id}>
        {backups
          ? row.resourceName
          : [formatActivityEvent(row.eventType), row.resourceName].filter(Boolean).join(' · ')}
      </Link>
      <span className="flex items-center gap-2">
        <ActivityActions id={row.id} />
        <StateBadge kind="activity" value={row.status} />
        <Timestamp value={row.createdAt} formatDateTime={format} />
      </span>
    </li>
  ));
}

function list(rows: ReactNode[] | undefined, empty: string, loading: boolean, error: unknown) {
  if (error || !rows) return <p className="text-muted-foreground">{loading && !error ? 'Loading…' : 'Unavailable'}</p>;
  return rows.length ? <ul className="divide-y text-sm">{rows}</ul> : <p className="text-muted-foreground">{empty}</p>;
}

function displayPercent(value: number | null | undefined) {
  return value != null && Number.isFinite(value) ? value.toFixed(1) + '%' : '—';
}

export function OverviewTiles({
  tiles,
  tags,
  range,
  manual,
}: {
  tiles: SavedTile[];
  tags: string[];
  range: number;
  manual: boolean;
}) {
  const app = useAppContext();
  const { groups } = useRealtimeContext();
  const platformLive = groups?.isGroupLive('platforms');
  const needsPlatforms =
    !tags.length && tiles.some((w) => w.settings.kind === 'Platforms' || w.settings.kind === 'ResourceSummary');
  const [fallback] = useState(() => 60000 + Math.floor(Math.random() * 5000));
  const platformRead = useRead('listPlatforms', undefined, {
    enabled: needsPlatforms && (manual || !platformLive),
    staleTime: manual ? 0 : fallback,
    refetchInterval: !manual && !platformLive ? fallback : false,
  });
  if (tags.length) return <TaggedTiles tiles={tiles} tags={tags} range={range} manual={manual} />;
  return (
    <TileGrid
      range={range}
      manual={manual}
      tiles={tiles}
      platforms={manual || !platformLive ? platformRead.data?.data.platforms : app.platforms}
      platformError={manual || !platformLive ? platformRead.error : app.platformListError}
      platformsLoading={manual || !platformLive ? platformRead.isLoading : app.isLoading}
    />
  );
}

function TaggedTiles({
  tiles,
  tags,
  range,
  manual,
}: {
  tiles: SavedTile[];
  tags: string[];
  range: number;
  manual: boolean;
}) {
  const { selectedTagNames, filterReady } = useResolvedTagFilter(tags);
  const needsPlatforms = tiles.some(
    ({ settings }) => settings.kind === 'Platforms' || settings.kind === 'ResourceSummary',
  );
  const scoped = usePlatformCollection(selectedTagNames, needsPlatforms && filterReady, !manual);
  return (
    <TileGrid
      range={range}
      manual={manual}
      tiles={tiles}
      tags={selectedTagNames}
      ready={filterReady}
      platforms={scoped.platformsMessage}
      platformError={scoped.error}
      platformsLoading={!filterReady || scoped.isLoading}
    />
  );
}

function TileGrid({
  range,
  manual,
  tiles,
  tags,
  ready = true,
  platforms,
  platformError,
  platformsLoading,
}: {
  range: number;
  manual: boolean;
  tiles: SavedTile[];
  tags?: string[];
  ready?: boolean;
  platforms?: PlatformView[];
  platformError?: unknown;
  platformsLoading: boolean;
}) {
  const app = useAppContext();
  const limits = useMemo(() => {
    const result: Record<string, number> = {};
    for (const tile of tiles)
      result[tile.settings.kind] = Math.max(
        result[tile.settings.kind] ?? 0,
        'limit' in tile.settings ? tile.settings.limit : 0,
      );
    return result;
  }, [tiles]);
  const alertQuery = useMemo(
    () =>
      ({
        page: 1,
        pageSize: Math.max(5, limits.OpenAlerts || 0),
        alertType: 'All',
        resourceId: undefined,
        resourceType: 'All',
        unresolvedOnly: true,
      }) as const,
    [limits.OpenAlerts],
  );
  const profile = useRead('getCurrentProfile', undefined, { enabled: !!limits.OpenAlerts });
  // The notification feed already owns the first five unresolved alerts and their live updates.
  const [interval] = useState(() => 60_000 + Math.floor(Math.random() * 5_000));
  const alerts = useAlertPage(
    alertQuery,
    {
      enabled: ready && (!!limits.OpenAlerts || (!!tags && 'ResourceSummary' in limits)),
      staleTime: !manual && !tags && alertQuery.pageSize === 5 ? Infinity : 0,
      refetchInterval: !manual && !app.alertFeedConnected ? interval : false,
    },
    tags,
  );
  // Global activity has no subscription. Only active tiles observe these bounded queries.
  const recent = useRead(
    'listRecentActivities',
    { query: { tags, limit: limits.RecentActivity || 5 } },
    {
      enabled: ready && !!limits.RecentActivity,
      refetchInterval: manual ? false : interval,
    },
  );
  const backups = useRead(
    'listRecentActivities',
    { query: { tags, eventType: ActivityEventType.BackupRunCompleted, limit: limits.Backups || 5 } },
    {
      enabled: ready && !!limits.Backups,
      refetchInterval: manual ? false : interval,
    },
  );
  const formatTime = useProfileDateTimeFormatter();

  function contents(tile: SavedTile) {
    const limit = 'limit' in tile.settings ? tile.settings.limit : 0;
    switch (tile.settings.kind) {
      case 'Platforms':
        return list(
          platforms?.slice(0, limit).map((platform) => {
            const sample = platform.status === PlatformStatus.Online ? platform.stats?.[0] : undefined;
            return (
              <li key={platform.id} className="flex flex-wrap items-center justify-between gap-2 py-1.5">
                <Link className="table-link" to={'/platforms/edit/' + platform.id}>
                  {platform.name}
                </Link>
                <span className="flex flex-wrap items-center gap-2">
                  <StateBadge value={platform.status} />
                  <span>CPU {displayPercent(sample?.cpuUsage)}</span>
                  <span>Memory {displayPercent(sample?.memoryUsage)}</span>
                  <Link
                    className="table-link"
                    to={monitoringLink('/platforms/edit/' + platform.id, 'Platform', 'stats', range)}>
                    Statistics
                  </Link>
                  <CopyResourceLink path={'/platforms/edit/' + platform.id} />
                </span>
              </li>
            );
          }),
          tags ? 'No visible platforms match these tags.' : 'No visible platforms.',
          platformsLoading,
          platformError,
        );
      case 'ResourceSummary': {
        const visiblePlatforms = platformError ? undefined : platforms;
        const containers = visiblePlatforms?.map(({ platformDescriptor }) =>
          platformDescriptor && 'containerCount' in platformDescriptor ? platformDescriptor.containerCount : undefined,
        );
        const count = containers?.every((n): n is number => typeof n === 'number')
          ? containers.reduce((a, b) => a + b, 0)
          : undefined;
        return (
          <dl className="flex flex-wrap gap-6">
            {Object.entries({
              Platforms: visiblePlatforms?.length,
              'Reported containers': count,
              'Open alerts': tags
                ? alerts.error
                  ? undefined
                  : alerts.pagedAlertEvents?.totalCount
                : app.alertCountAvailable
                  ? app.unresolvedAlertCount
                  : undefined,
            }).map(([name, value]) => (
              <div key={name}>
                <dt className="text-xs text-muted-foreground">{name}</dt>
                <dd className="mt-1 text-xl tabular-nums">{value ?? '—'}</dd>
              </div>
            ))}
          </dl>
        );
      }
      case 'OpenAlerts':
        return list(
          alerts.pagedAlertEvents?.items
            .slice(0, limit)
            .map((row) => (
              <AlertRow key={row.id} row={row} auth={profile.isError ? undefined : profile.data?.data.authorization} />
            )),
          'No open alerts.',
          !ready || alerts.isLoading,
          alerts.error,
        );
      case 'Backups':
      case 'RecentActivity': {
        const isBackup = tile.settings.kind === 'Backups';
        const query = isBackup ? backups : recent;
        return list(
          query.data && recentRows(query.data.data, limit, isBackup, formatTime),
          isBackup ? 'No completed backups.' : 'No recent activity.',
          !ready || query.isLoading,
          query.error,
        );
      }
      default:
        return <p className="text-muted-foreground">This widget is unavailable.</p>;
    }
  }
  return (
    <OrderedTiles tiles={tiles}>
      {tiles
        .filter((tile) =>
          ['Platforms', 'ResourceSummary', 'OpenAlerts', 'Backups', 'RecentActivity'].includes(tile.settings.kind),
        )
        .map((tile) => (
          <Tile key={tile.id} tile={tile}>
            {contents(tile)}
          </Tile>
        ))}
      <SupplementalTiles
        tiles={tiles}
        tags={tags ?? []}
        range={range}
        enabled={ready}
        interval={manual ? false : interval}
      />
    </OrderedTiles>
  );
}
