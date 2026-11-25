import Loader from '@/components/ui/loader';
import { useMemo, useState, type ReactNode } from 'react';
import { Link } from 'react-router';
import { ActivityEventType, type PlatformView, type RecentActivityView } from '@/api/generated/api.types';
import { StateBadge } from '@/components/custom/state-badge';
import { useAlertPage } from '@/features/alerters/alert-events/hooks/useAlertEventsList';
import { useAppContext } from '@/lib/context/app-context';
import { useRealtimeContext } from '@/lib/context/realtime-context';
import { useRead } from '@/lib/hooks';
import { formatActivityEvent } from '@/lib/utils';
import { PlatformRow } from './resource-rows';
import { TimelineItem } from './timeline';
import { CitadelIcons } from '@/lib/icons';
import { usePlatformCollection } from '@/features/platforms/hooks/usePlatformsGroup';
import { useResolvedTagFilter } from '@/features/tags/components';
import { Tile, OrderedTiles, WidgetNotice } from './tile';
import { SupplementalTiles } from './widget-data';
import { ActivityActions } from './row-actions';
import { AlertRow } from './alert-row';
import type { DashboardRequirements } from './requirements';
import type { SavedTile } from './grid';

import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import type { DateTimeFormatter } from '@/lib/date-time';
function recentRows(rows: RecentActivityView[], limit: number, backups: boolean, format: DateTimeFormatter) {
  return rows
    .slice(0, limit)
    .map((row) => (
      <TimelineItem
        format={format}
        key={row.id}
        type={row.resourceType}
        title={
          backups
            ? row.resourceName
            : [formatActivityEvent(row.eventType), row.resourceName].filter(Boolean).join(' · ')
        }
        path={'/activities/' + row.id}
        time={row.createdAt}
        status={<StateBadge kind="activity" value={row.status} />}
        actions={<ActivityActions id={row.id} />}
      />
    ));
}

function list(rows: ReactNode[] | undefined, empty: string, loading: boolean, error: unknown, timeline = false) {
  if (error || !rows) return loading && !error ? <Loader /> : <WidgetNotice>Unavailable</WidgetNotice>;
  return rows.length ? (
    <ul className={timeline ? 'text-sm' : 'divide-y text-sm'}>{rows}</ul>
  ) : (
    <p className="text-muted-foreground">{empty}</p>
  );
}

export function OverviewTiles({
  plan,
  tiles,
  tags,
  range,
  manual,
}: {
  plan: DashboardRequirements;
  tiles: SavedTile[];
  tags: string[];
  range: number;
  manual: boolean;
}) {
  const app = useAppContext();
  const { groups } = useRealtimeContext();
  const platformLive = groups?.isGroupLive('platforms');
  const needsPlatforms = !tags.length && plan.needsPlatforms;
  const [fallback] = useState(() => 60000 + Math.floor(Math.random() * 5000));
  const platformRead = useRead('listPlatforms', undefined, {
    enabled: needsPlatforms && (manual || !platformLive),
    staleTime: manual ? 0 : fallback,
    refetchInterval: !manual && !platformLive ? fallback : false,
  });
  if (tags.length) return <TaggedTiles plan={plan} tiles={tiles} tags={tags} range={range} manual={manual} />;
  return (
    <TileGrid
      plan={plan}
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
  plan,
  tiles,
  tags,
  range,
  manual,
}: {
  plan: DashboardRequirements;
  tiles: SavedTile[];
  tags: string[];
  range: number;
  manual: boolean;
}) {
  const { selectedTagNames, filterReady } = useResolvedTagFilter(tags);
  const scoped = usePlatformCollection(selectedTagNames, plan.needsPlatforms && filterReady, !manual);
  return (
    <TileGrid
      plan={plan}
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
  plan,
  tiles,
  tags,
  ready = true,
  platforms,
  platformError,
  platformsLoading,
}: {
  range: number;
  manual: boolean;
  plan: DashboardRequirements;
  tiles: SavedTile[];
  tags?: string[];
  ready?: boolean;
  platforms?: PlatformView[];
  platformError?: unknown;
  platformsLoading: boolean;
}) {
  const app = useAppContext();
  const format = useProfileDateTimeFormatter();
  const { limits } = plan;
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
  const query = new URLSearchParams(tags?.map((tag) => ['tags', tag]));
  const scopedPath = (path: string) => path + (query.size ? '?' + query : '');

  function contents(tile: SavedTile) {
    const limit = 'limit' in tile.settings ? tile.settings.limit : 0;
    switch (tile.settings.kind) {
      case 'Platforms':
        return list(
          platforms
            ?.slice(0, limit)
            .map((platform) => <PlatformRow key={platform.id} platform={platform} range={range} />),
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
          <dl className="grid grid-cols-3 divide-x py-1">
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
            }).map(([name, value]) => {
              const Icon =
                name === 'Platforms'
                  ? CitadelIcons.Platform
                  : name === 'Open alerts'
                    ? CitadelIcons.Alert
                    : CitadelIcons.Container;
              const path =
                name === 'Platforms'
                  ? scopedPath('/platforms')
                  : name === 'Open alerts' && !tags?.length
                    ? '/alerts'
                    : undefined;
              return (
                <div key={name} className="min-w-0 px-2 first:pl-0 sm:px-4">
                  <dt className="flex items-center gap-1.5 text-xs text-muted-foreground">
                    <Icon className="hidden size-3.5 sm:block" />
                    {name}
                  </dt>
                  <dd className="mt-1 text-xl font-semibold tabular-nums">
                    {path ? (
                      <Link aria-label={name + ' · ' + (value ?? 'unavailable')} to={path}>
                        {value ?? '—'}
                      </Link>
                    ) : (
                      (value ?? '—')
                    )}
                    {name === 'Platforms' && visiblePlatforms && (
                      <span className="ml-2 whitespace-nowrap text-xs font-normal text-muted-foreground">
                        {visiblePlatforms.filter((p) => p.status === 'Online').length} online
                      </span>
                    )}
                  </dd>
                </div>
              );
            })}
          </dl>
        );
      }
      case 'OpenAlerts':
        return list(
          alerts.pagedAlertEvents?.items
            .slice(0, limit)
            .map((row) => (
              <AlertRow
                format={format}
                key={row.id}
                row={row}
                auth={profile.isError ? undefined : profile.data?.data.authorization}
              />
            )),
          'No open alerts.',
          !ready || alerts.isLoading,
          alerts.error,
          true,
        );
      case 'Backups':
      case 'RecentActivity': {
        const isBackup = tile.settings.kind === 'Backups';
        const query = isBackup ? backups : recent;
        if (!query.isError && query.data?.data.length === 0) {
          const Icon = isBackup ? CitadelIcons.BackupPolicy : CitadelIcons.Activity;
          return (
            <div className="flex flex-1 flex-col items-center justify-center gap-2 py-4 text-center text-sm text-muted-foreground">
              <span className="flex size-10 items-center justify-center rounded-full bg-muted">
                <Icon className="size-5" />
              </span>
              {isBackup ? 'No completed backups.' : 'No recent activity.'}
            </div>
          );
        }
        return list(
          query.data && recentRows(query.data.data, limit, isBackup, format),
          isBackup ? 'No completed backups.' : 'No recent activity.',
          !ready || query.isLoading,
          query.error,
          true,
        );
      }
      default:
        return <p className="text-muted-foreground">This widget is unavailable.</p>;
    }
  }
  return (
    <OrderedTiles tiles={tiles}>
      {plan.overview.map((tile) => (
        <Tile key={tile.id} tile={tile}>
          {contents(tile)}
          {tile.settings.kind === 'Platforms' && (
            <Link className="table-link mt-1 text-xs" to={scopedPath('/platforms')}>
              View all platforms
            </Link>
          )}
          {tile.settings.kind === 'RecentActivity' && (
            <Link className="table-link mt-auto text-xs" to="/activities">
              View all activities
            </Link>
          )}
          {tile.settings.kind === 'OpenAlerts' && (
            <Link className="table-link mt-auto text-xs" to="/alerts">
              View all alerts
            </Link>
          )}
        </Tile>
      ))}
      <SupplementalTiles
        plan={plan}
        tags={tags ?? []}
        range={range}
        enabled={ready}
        interval={manual ? false : interval}
      />
    </OrderedTiles>
  );
}
