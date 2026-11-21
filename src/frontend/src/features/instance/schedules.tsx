import { useCallback, useState } from 'react';
import { Link } from 'react-router';
import { CalendarClock } from 'lucide-react';
import type { ScheduleView } from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { CommandGroup, CommandItem } from '@/components/ui/command';
import { ResourceFilterDropdown } from '@/components/custom/resource-filter';
import { Table, TableHeader, TableRow, TableHead, TableBody, TableCell } from '@/components/ui/table';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import type { RealtimeConnection } from '@/lib/realtime-connection';
import { useRead } from '@/lib/hooks';
import { formatId } from '@/lib/utils';

const purposes: Record<string, [string, string, string]> = {
  Script: ['Script', 'Run script', 'scripts'],
  Backup: ['Backup policy', 'Run backup', 'backup-policies'],
  Install: ['Instance', 'Install Core and Agent updates', ''],
  Release: ['Instance', 'Check for Core releases', ''],
  Git: ['Git repository', 'Synchronize branches', 'git-repos'],
  DeploymentImages: ['Deployment', 'Check image updates', 'deployments'],
  ServiceImages: ['Swarm service', 'Check image updates', 'swarm-services'],
  StackImages: ['Stack', 'Check image updates', 'stacks'],
  StackGit: ['Stack', 'Reconcile Git changes', 'stacks'],
  Image: ['Image', 'Check registry tags', ''],
};
function resourceLink(s: ScheduleView) {
  if (s.purpose === 'Image')
    return s.platformId && s.dockerImageId
      ? `/platforms/${s.platformId}/images/${formatId(s.dockerImageId)}`
      : undefined;
  if (s.purpose === 'Install') return '/updates';
  if (s.purpose === 'Release') return '/instance/config';
  const resource = purposes[s.purpose]?.[2];
  return resource ? `/${resource}/edit/${s.ownerId}` : undefined;
}
function recurrence(s: ScheduleView) {
  if (s.cron) {
    if (s.cron === '* * * * *') return 'Every minute';
    const daily = /^(\d+) (\d+) \* \* \*$/.exec(s.cron);
    if (daily) return `Daily at ${daily[2].padStart(2, '0')}:${daily[1].padStart(2, '0')}`;
    const step = /^\*\/(\d+) \* \* \* \*$/.exec(s.cron);
    if (step) return `Every ${step[1]} minutes`;
    const hourly = /^(\d+) \* \* \* \*$/.exec(s.cron);
    if (hourly) return `Hourly at :${hourly[1].padStart(2, '0')}`;
    const weekly = /^(\d+) (\d+) \* \* ([0-7])$/.exec(s.cron);
    if (weekly) {
      const days = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
      return `Every ${days[Number(weekly[3]) % 7]} at ${weekly[2].padStart(2, '0')}:${weekly[1].padStart(2, '0')}`;
    }
    return 'Custom schedule';
  }
  if (!s.intervalSeconds) return 'Registry check deadline';
  const minutes = s.intervalSeconds / 60;
  return minutes % 60 === 0 ? `Every ${minutes / 60} hour${minutes === 60 ? '' : 's'}` : `Every ${minutes} min`;
}

export function InstanceSchedules() {
  const [purpose, setPurpose] = useState<string>();
  const [pages, setPages] = useState<string[]>([]);
  const [open, setOpen] = useState(false);
  const query = useRead(
    'getInstanceSchedules',
    { query: { purpose, after: pages.at(-1) } },
    { refetchOnMount: 'always', retry: false },
  );
  const { refetch } = query;
  const refresh = useCallback(() => {
    void refetch();
  }, [refetch]);
  const listen = useCallback((c: RealtimeConnection) => c.on('InstanceSchedulesChanged', refresh), [refresh]);
  const unlisten = useCallback((c: RealtimeConnection) => c.off('InstanceSchedulesChanged', refresh), [refresh]);
  const start = useCallback(
    (c: RealtimeConnection) => {
      void c.invoke('StartInstanceSchedules').catch(refresh);
    },
    [refresh],
  );
  useRealtimeGroup({
    groupName: 'instance-schedules',
    setupEventListeners: listen,
    removeEventListeners: unlisten,
    onJoinedGroup: start,
    onSubscriptionError: refresh,
  });
  const data = query.isError ? undefined : query.data?.data;
  const select = (value?: string) => {
    setPurpose(value);
    setPages([]);
    setOpen(false);
  };
  return (
    <section className="space-y-3">
      <ResourceFilterDropdown
        open={open}
        onOpenChange={setOpen}
        label={purpose ? (purposes[purpose]?.slice(0, 2).join(' · ') ?? purpose) : 'Schedule filter'}
        icon={CalendarClock}
        searchPlaceholder="Find a schedule type…"
        emptyMessage="No matching types.">
        <CommandGroup>
          <CommandItem onSelect={() => select()}>All schedules</CommandItem>
          {Object.entries(purposes).map(([key, [resource, action]]) => (
            <CommandItem key={key} onSelect={() => select(key)}>
              {resource} · {action}
            </CommandItem>
          ))}
        </CommandGroup>
      </ResourceFilterDropdown>
      <Table>
        <TableHeader>
          <TableRow>
            {['Resource', 'Purpose', 'Recurrence', 'Time zone', 'Next scheduled time (local)', 'State'].map((label) => (
              <TableHead key={label}>{label}</TableHead>
            ))}
          </TableRow>
        </TableHeader>
        <TableBody>
          {data?.items.map((s) => {
            const link = resourceLink(s);
            return (
              <TableRow key={s.purpose + s.ownerId}>
                <TableCell>
                  {link ? (
                    <Link className="table-link" to={link}>
                      {s.name}
                    </Link>
                  ) : (
                    s.name
                  )}
                  <div className="text-xs text-muted-foreground">{purposes[s.purpose]?.[0]}</div>
                </TableCell>
                <TableCell>{purposes[s.purpose]?.[1] ?? s.purpose}</TableCell>
                <TableCell>
                  {recurrence(s)}
                  {s.cron && <div className="font-mono text-xs text-muted-foreground">{s.cron}</div>}
                  {s.systemManaged && <div className="text-xs text-muted-foreground">System managed</div>}
                </TableCell>
                <TableCell>{s.timeZone ?? '—'}</TableCell>
                <TableCell>{s.nextAt ? new Date(s.nextAt).toLocaleString() : '—'}</TableCell>
                <TableCell>
                  {!s.enabled ? 'Disabled' : s.nextAt ? 'Enabled' : 'Unavailable'}
                  {s.reason && s.enabled && <div className="text-xs text-muted-foreground">{s.reason}</div>}
                </TableCell>
              </TableRow>
            );
          })}
          {!data?.items.length && (
            <TableRow>
              <TableCell colSpan={6}>
                {query.isError
                  ? 'Schedule catalog unavailable.'
                  : query.isPending
                    ? 'Loading schedules…'
                    : 'No schedules match this filter.'}
              </TableCell>
            </TableRow>
          )}
        </TableBody>
      </Table>
      {(pages.length > 0 || data?.nextCursor) && (
        <div className="flex justify-end gap-2">
          <Button variant="outline" disabled={!pages.length} onClick={() => setPages((p) => p.slice(0, -1))}>
            Previous
          </Button>
          <Button
            variant="outline"
            disabled={!data?.nextCursor}
            onClick={() => {
              if (data?.nextCursor) setPages([...pages, data.nextCursor]);
            }}>
            Next
          </Button>
        </div>
      )}
    </section>
  );
}
