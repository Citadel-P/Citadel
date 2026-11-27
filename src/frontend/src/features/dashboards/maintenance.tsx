import { useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useScheduleChanges } from '@/features/instance/schedules/useSchedules';
import { ArrowRight, ArrowUpRight, Database, Server } from 'lucide-react';
import { Link } from 'react-router';
import type { DashboardWidget, ScheduleView, UpdateStatusView } from '@/api/generated/api.types';
import { Timestamp } from '@/components/custom/timestamp';
import { Button } from '@/components/ui/button';
import { purposes, resourceLink } from '@/features/instance/schedules/presentation';
import { CoreStatusBadge, DatabaseStatusBadge } from '@/features/updates/status-badge';
import { type DateTimeFormatter } from '@/lib/date-time';
import { useRealtimeContext } from '@/lib/context/realtime-context';
import { useRead } from '@/lib/hooks';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { Tile, WidgetNotice, WidgetSkeleton } from './tile';
import { TimelineItem } from './timeline';

const scheduleIcons: Record<string, string> = {
  Script: 'Script',
  Backup: 'BackupPolicy',
  Install: 'Updates',
  Release: 'Updates',
  Git: 'GitRepository',
  DeploymentImages: 'Deployment',
  ServiceImages: 'SwarmService',
  StackImages: 'Stack',
  StackGit: 'Stack',
  Image: 'Image',
};

function ScheduleTimeline({ items, format }: { items: ScheduleView[]; format: DateTimeFormatter }) {
  return items.length ? (
    <ul>
      {items.map((item) => (
        <TimelineItem
          key={item.purpose + item.ownerId}
          type={scheduleIcons[item.purpose] ?? 'Instance'}
          title={purposes[item.purpose]?.[1] ?? item.name}
          resourceName={purposes[item.purpose]?.[2] || item.purpose === 'Image' ? item.name : undefined}
          to={resourceLink(item)}
          time={item.nextAt ?? undefined}
          format={format}
          tone={item.needsAttention ? 'warning' : 'info'}
          compact
        />
      ))}
    </ul>
  ) : (
    <p className="m-auto py-4 text-center text-muted-foreground">No upcoming enabled schedules.</p>
  );
}

function UpdateDetails({ status, format }: { status: UpdateStatusView; format: DateTimeFormatter }) {
  return (
    <>
      <dl className="divide-y">
        <div className="grid grid-cols-[auto_minmax(0,1fr)] gap-x-2.5 gap-y-1 py-2.5">
          <Server aria-hidden="true" className="mt-0.5 size-4 text-primary" />
          <dt className="flex flex-wrap items-center justify-between gap-1 font-medium">
            Citadel Core <CoreStatusBadge status={status} />
          </dt>
          <dd className="col-start-2 flex min-w-0 flex-wrap items-center gap-x-2 gap-y-0.5 text-xs text-muted-foreground">
            <span className="min-w-0 break-all font-mono" title="Installed version">
              {status.installedVersion}
            </span>
            {status.available && (
              <span className="flex min-w-0 items-center gap-1 text-info">
                <ArrowUpRight aria-hidden="true" className="size-3.5 shrink-0" />
                <span className="break-all">Available {status.available.version}</span>
              </span>
            )}
          </dd>
        </div>
        <div className="grid grid-cols-[auto_minmax(0,1fr)] gap-x-2.5 gap-y-1 py-2.5">
          <Database aria-hidden="true" className="mt-0.5 size-4 text-primary" />
          <dt className="flex flex-wrap items-center justify-between gap-1 font-medium">
            PostgreSQL <DatabaseStatusBadge status={status} />
          </dt>
          {(status.databaseVersion || status.databaseAvailable) && (
            <dd className="col-start-2 flex flex-wrap gap-x-2 text-xs text-muted-foreground">
              {status.databaseVersion && (
                <span className="font-mono" title="Installed version">
                  {status.databaseVersion}
                </span>
              )}
              {status.databaseAvailable && <span className="text-info">Available {status.databaseAvailable}</span>}
            </dd>
          )}
        </div>
      </dl>
      <div className="mt-1 flex flex-wrap items-center gap-x-1.5 text-xs text-muted-foreground">
        {status.lastSuccess && (
          <>
            Last successful Core check{' '}
            <Timestamp className="text-xs text-muted-foreground" value={status.lastSuccess} formatDateTime={format} />
          </>
        )}
        {!status.checksEnabled && <span>Automatic Core checks disabled</span>}
      </div>
    </>
  );
}

export function MaintenanceTiles({
  tiles,
  tags,
  enabled,
  interval,
}: {
  tiles: DashboardWidget[];
  tags: string[];
  enabled: boolean;
  interval: number | false;
}) {
  const profile = useRead('getCurrentProfile');
  const admin = !profile.isError && profile.data?.data.authorization.isAdministrator === true;
  const limit = Math.max(
    0,
    ...tiles.map(({ settings }) => (settings.kind === 'UpcomingSchedules' ? settings.limit : 0)),
  );
  const client = useQueryClient();
  const catalogChanged = useCallback(() => {
    void client.invalidateQueries({ queryKey: ['getUpcomingSchedules'] });
  }, [client]);
  const observe = admin && enabled && limit > 0;
  const { synchronized } = useScheduleChanges(observe, catalogChanged);
  const schedules = useRead(
    'getUpcomingSchedules',
    { query: { tags, limit } },
    { enabled: observe, refetchInterval: synchronized ? false : interval },
  );
  const { liveConnectionState } = useRealtimeContext();
  const updates = useRead('getInstanceUpdates', undefined, {
    enabled: admin && enabled && tiles.some(({ settings }) => settings.kind === 'Updates'),
    staleTime: interval || 0,
    refetchInterval: liveConnectionState === 'connected' ? false : interval,
  });
  const format = useProfileDateTimeFormatter();
  return tiles.map((tile) => {
    if (!admin)
      return (
        <Tile key={tile.id} tile={tile} pending={profile.isLoading}>
          {profile.isLoading ? (
            <WidgetSkeleton />
          ) : profile.isError ? (
            <WidgetNotice>Unavailable</WidgetNotice>
          ) : (
            <p>Administrator access required.</p>
          )}
        </Tile>
      );
    const isUpdates = tile.settings.kind === 'Updates';
    const query = isUpdates ? updates : schedules;
    const pending = !enabled || query.isLoading;
    let body;
    if (query.isError)
      body = (
        <WidgetNotice>{isUpdates ? 'Update status unavailable.' : 'Upcoming schedules unavailable.'}</WidgetNotice>
      );
    else if (isUpdates && updates.data) body = <UpdateDetails status={updates.data.data} format={format} />;
    else if (tile.settings.kind === 'UpcomingSchedules' && schedules.data)
      body = <ScheduleTimeline items={schedules.data.data.items.slice(0, tile.settings.limit)} format={format} />;
    else body = <WidgetSkeleton />;
    return (
      <Tile key={tile.id} tile={tile} pending={pending}>
        <div className="dashboard-timeline-list px-4 py-3">{body}</div>
        <Button
          className="dashboard-widget-footer shrink-0 text-muted-foreground hover:text-foreground"
          variant="ghost"
          size="sm"
          asChild>
          <Link to={isUpdates ? '/updates' : '/instance/schedules'}>
            {isUpdates ? 'View updates' : 'View schedules'} <ArrowRight aria-hidden="true" className="size-3.5" />
          </Link>
        </Button>
      </Tile>
    );
  });
}
