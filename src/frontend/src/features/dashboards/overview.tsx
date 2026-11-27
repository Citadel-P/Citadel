import { useDashboardRead } from './use-dashboard-read';
import { useMemo, useState, type ReactNode } from 'react';
import { ArrowRight } from 'lucide-react';
import { Link } from 'react-router';
import { useOpenActivitySheet } from '@/features/alerters/alert-events/alert-task-sheet';
import { ActivityEventType, type RecentActivityView } from '@/api/generated/api.types';
import { StateBadge } from '@/components/custom/state-badge';
import { useAlertPage } from '@/features/alerters/alert-events/hooks/useAlertEventsList';
import { useAppContext } from '@/lib/context/app-context';
import { useRead } from '@/lib/hooks';
import { cn, formatActivityEvent } from '@/lib/utils';
import { TimelineItem } from './timeline';
import { CitadelIcons } from '@/lib/icons';
import { useResolvedTagFilter } from '@/features/tags/components';
import { Tile, WidgetNotice, WidgetSkeleton } from './tile';
import { SupplementalTiles } from './widget-data';
import { ActivityActions } from './row-actions';
import { Sheet, SheetTrigger, SheetContent, SheetHeader, SheetTitle } from '@/components/ui/sheet';
import { Button } from '@/components/ui/button';
import { AlertRow, ScopedAlerts } from './alert-row';
import type { DashboardRequirements } from './requirements';
import type { SavedTile } from './grid';

import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import type { DateTimeFormatter } from '@/lib/date-time';
function recentRows(
  rows: RecentActivityView[],
  limit: number,
  backups: boolean,
  format: DateTimeFormatter,
  open: (id: string) => void,
) {
  return rows
    .slice(0, limit)
    .map((row) => (
      <TimelineItem
        format={format}
        key={row.id}
        type={row.resourceType}
        title={backups ? row.resourceName : formatActivityEvent(row.eventType)}
        resourceName={backups ? undefined : row.resourceName}
        onOpen={() => open(row.id)}
        compact={!backups}
        tone={
          row.status === 'Failure'
            ? 'failure'
            : row.status === 'Warning'
              ? 'warning'
              : row.status === 'Success'
                ? 'success'
                : row.status === 'Information'
                  ? 'info'
                  : 'neutral'
        }
        time={row.createdAt}
        statusLabel={backups ? undefined : row.status}
        status={backups && <StateBadge kind="activity" value={row.status} />}
        actions={backups && <ActivityActions id={row.id} />}
      />
    ));
}

function list(rows: ReactNode[] | undefined, empty: string, loading: boolean, error: unknown) {
  if (error || !rows) return loading && !error ? <WidgetSkeleton /> : <WidgetNotice>Unavailable</WidgetNotice>;
  return rows.length ? <ul className="text-sm">{rows}</ul> : <p className="text-muted-foreground">{empty}</p>;
}

function ScopedActivity({ tags }: { tags: string[] }) {
  const format = useProfileDateTimeFormatter();
  const openActivity = useOpenActivitySheet();
  const recent = useDashboardRead('listRecentActivities', { query: { tags, limit: 10 } });
  return (
    <div className="min-h-0 overflow-auto px-4">
      {list(
        recent.data && recentRows(recent.data.data, 10, false, format, openActivity),
        'No recent activity.',
        recent.isLoading,
        recent.error,
      )}
    </div>
  );
}

export function OverviewTiles({
  plan,
  tags: tokens,
  range,
}: {
  plan: DashboardRequirements;
  tags: string[];
  range: number;
}) {
  const { selectedTagNames, filterReady: ready } = useResolvedTagFilter(tokens);
  const tags = selectedTagNames.length ? selectedTagNames : undefined;
  const app = useAppContext();
  const format = useProfileDateTimeFormatter();
  const openActivity = useOpenActivitySheet();
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
      staleTime: !tags && alertQuery.pageSize === 5 ? Infinity : 0,
      refetchInterval: !app.alertFeedConnected ? interval : false,
    },
    tags,
  );
  const recent = useDashboardRead(
    'listRecentActivities',
    { query: { tags, limit: limits.RecentActivity || 5 } },
    {
      enabled: ready && !!limits.RecentActivity,
    },
  );
  const backups = useDashboardRead(
    'listRecentActivities',
    { query: { tags, eventType: ActivityEventType.BackupRunCompleted, limit: limits.Backups || 5 } },
    {
      enabled: ready && !!limits.Backups,
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
          'No open alerts in this scope.',
          !ready || alerts.isLoading,
          alerts.error,
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
          query.data && recentRows(query.data.data, limit, isBackup, format, openActivity),
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
    <>
      {plan.overview.map((tile) => (
        <Tile key={tile.id} tile={tile}>
          {tile.settings.kind === 'RecentActivity' || tile.settings.kind === 'Backups' ? (
            <div
              className={cn(
                'dashboard-timeline-list',
                tile.settings.kind === 'RecentActivity'
                  ? 'px-5 py-2 [&_.dashboard-timeline-label]:text-[0.8125rem] [&_.dashboard-timeline-label]:font-normal'
                  : 'px-4 py-3',
              )}>
              {contents(tile)}
            </div>
          ) : (
            contents(tile)
          )}
          {tile.settings.kind === 'RecentActivity' && (
            <Sheet>
              <SheetTrigger asChild>
                <Button
                  className="dashboard-widget-footer shrink-0 text-muted-foreground hover:text-foreground"
                  variant="ghost"
                  size="sm">
                  View recent activity <ArrowRight aria-hidden="true" className="size-3.5" />
                </Button>
              </SheetTrigger>
              <SheetContent>
                <SheetHeader>
                  <SheetTitle>Recent activity</SheetTitle>
                </SheetHeader>
                <ScopedActivity tags={tags ?? []} />
              </SheetContent>
            </Sheet>
          )}
          {tile.settings.kind === 'Backups' && (
            <Button
              className="dashboard-widget-footer shrink-0 text-muted-foreground hover:text-foreground"
              variant="ghost"
              size="sm"
              asChild>
              <Link to="/backup-policies">
                View backup policies <ArrowRight aria-hidden="true" className="size-3.5" />
              </Link>
            </Button>
          )}
          {tile.settings.kind === 'OpenAlerts' && <ScopedAlerts tags={tags ?? []}>View all alerts</ScopedAlerts>}
        </Tile>
      ))}
      <SupplementalTiles plan={plan} tags={tags ?? []} range={range} enabled={ready} interval={interval} />
    </>
  );
}
