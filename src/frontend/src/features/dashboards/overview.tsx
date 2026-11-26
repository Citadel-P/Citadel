import Loader from '@/components/ui/loader';
import { useMemo, useState, type ReactNode } from 'react';
import { Link } from 'react-router';
import { ActivityEventType, type RecentActivityView } from '@/api/generated/api.types';
import { StateBadge } from '@/components/custom/state-badge';
import { useAlertPage } from '@/features/alerters/alert-events/hooks/useAlertEventsList';
import { useAppContext } from '@/lib/context/app-context';
import { useRead } from '@/lib/hooks';
import { formatActivityEvent } from '@/lib/utils';
import { TimelineItem } from './timeline';
import { CitadelIcons } from '@/lib/icons';
import { useResolvedTagFilter } from '@/features/tags/components';
import { Tile, OrderedTiles, WidgetNotice } from './tile';
import { SupplementalTiles } from './widget-data';
import { ActivityActions } from './row-actions';
import { Sheet, SheetTrigger, SheetContent, SheetHeader, SheetTitle } from '@/components/ui/sheet';
import { Button } from '@/components/ui/button';
import { AlertRow, ScopedAlerts } from './alert-row';
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
        failed={row.status === 'Failure'}
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

function ScopedActivity({ tags }: { tags: string[] }) {
  const format = useProfileDateTimeFormatter();
  const recent = useRead('listRecentActivities', { query: { tags, limit: 10 } });
  return (
    <div className="min-h-0 overflow-auto px-4">
      {list(
        recent.data && recentRows(recent.data.data, 10, false, format),
        'No recent activity.',
        recent.isLoading,
        recent.error,
        true,
      )}
    </div>
  );
}

export function OverviewTiles({
  plan,
  tiles,
  tags: tokens,
  range,
  manual,
}: {
  plan: DashboardRequirements;
  tiles: SavedTile[];
  tags: string[];
  range: number;
  manual: boolean;
}) {
  const { selectedTagNames, filterReady: ready } = useResolvedTagFilter(tokens);
  const tags = selectedTagNames.length ? selectedTagNames : undefined;
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
      enabled: ready && !!limits.OpenAlerts,
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
  function contents(tile: SavedTile) {
    const limit = 'limit' in tile.settings ? tile.settings.limit : 0;
    switch (tile.settings.kind) {
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
          {tile.settings.kind === 'RecentActivity' &&
            (tags?.length ? (
              <Sheet>
                <SheetTrigger asChild>
                  <Button className="mt-auto w-fit" variant="link">
                    Open recent activity
                  </Button>
                </SheetTrigger>
                <SheetContent>
                  <SheetHeader>
                    <SheetTitle>Recent activity</SheetTitle>
                  </SheetHeader>
                  <ScopedActivity tags={tags} />
                </SheetContent>
              </Sheet>
            ) : (
              <Link className="table-link mt-auto text-xs" to="/activities">
                View all activities
              </Link>
            ))}
          {tile.settings.kind === 'OpenAlerts' &&
            (tags?.length ? (
              <ScopedAlerts tags={tags}>View all alerts</ScopedAlerts>
            ) : (
              <Link className="table-link mt-auto text-xs" to="/alerts">
                View all alerts
              </Link>
            ))}
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
