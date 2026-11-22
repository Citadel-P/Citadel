import { ChevronRight } from 'lucide-react';
import { Link } from 'react-router';
import { useState } from 'react';
import { useRead } from '@/lib/hooks';
import { PaginationControls } from '@/components/custom/common';
import { StateBadge } from '@/components/custom/state-badge';
import { StateIndicator } from '@/components/custom/state-indicator';
import { Timestamp } from '@/components/custom/timestamp';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { TableBody, TableCell, TableRow } from '@/components/ui/table';
import type { DateTimeFormatter } from '@/lib/date-time';
import { purposes, recurrence, resourceLink, scheduleState } from './presentation';

const PAGE_SIZE = 50;

export function ScheduleGroup({
  resource,
  total,
  attention,
  attentionOnly,
  label,
  synchronized,
  search,
  filtered,
  activeBatches,
  formatDateTime,
}: {
  resource: string;
  total: number;
  attention: number;
  attentionOnly: boolean;
  label: string;
  synchronized: boolean;
  search: string;
  filtered: boolean;
  activeBatches: string[];
  formatDateTime: DateTimeFormatter;
}) {
  const [expanded, setExpanded] = useState<boolean>();
  const [requestedPage, setPage] = useState(1);
  const count = attentionOnly ? attention : total;
  const totalPages = Math.max(1, Math.ceil(count / PAGE_SIZE));
  const page = Math.min(requestedPage, totalPages);
  if (requestedPage !== page) setPage(page);
  const open = expanded ?? filtered;
  const query = useRead(
    'getInstanceSchedules',
    {
      query: {
        resource,
        page: page === 1 ? undefined : page,
        pageSize: PAGE_SIZE,
        attentionOnly: attentionOnly || undefined,
        search: search || undefined,
      },
    },
    { enabled: open, retry: false, staleTime: synchronized ? Infinity : 30_000 },
  );
  const schedules = query.isError ? [] : (query.data?.data?.items ?? []);
  const id = `schedule-group-${resource}`;
  const counts = { 'Next run unknown': 0, Blocked: 0, Enabled: 0, Disabled: 0 };
  for (const schedule of schedules) counts[scheduleState(schedule)]++;
  return (
    <>
      <TableBody className="border-b border-border">
        <TableRow>
          <TableCell colSpan={7} className="p-0">
            <Button
              variant="ghost"
              size="sm"
              className="h-auto min-h-(--table-row-height) w-full flex-wrap justify-start gap-2 rounded-none px-3 py-(--table-cell-padding-y) font-normal"
              aria-label={`${label} schedules`}
              aria-expanded={open}
              aria-controls={id}
              aria-describedby={id + '-summary'}
              onClick={() => setExpanded(!open)}>
              <ChevronRight aria-hidden className={`size-4 shrink-0 ${open ? 'rotate-90' : ''}`} />
              {label}
              <span className="text-xs text-muted-foreground tabular-nums">
                {attentionOnly ? `${attention} of ${total}` : total} {total === 1 ? 'schedule' : 'schedules'}
              </span>
              <span id={id + '-summary'} className="ml-auto flex flex-wrap gap-1">
                {attention > 0 && <StateBadge value="Warning" label={`${attention} blocked`} />}
                {activeBatches.map((purpose) => (
                  <StateBadge key={purpose} value="Running" kind="run" label={`${purposes[purpose][1]} · Running`} />
                ))}
                {open &&
                  Object.entries(counts).map(([state, count]) =>
                    count > 0 && state !== 'Blocked' ? (
                      <StateBadge
                        key={state}
                        value={state}
                        label={`${count} ${state.toLowerCase()}${totalPages > 1 ? ' on this page' : ''}`}
                      />
                    ) : null,
                  )}
              </span>
            </Button>
          </TableCell>
        </TableRow>
      </TableBody>
      <TableBody id={id} hidden={!open}>
        {open &&
          schedules.map((s) => {
            const state = scheduleState(s);
            const link = resourceLink(s);
            const status = s.resourceStatus;
            const resourceLabel = status ? `${label} ${status.toLowerCase()}` : undefined;
            return (
              <TableRow key={s.purpose + s.ownerId}>
                <TableCell>
                  <div className="flex items-center gap-3 pl-6">
                    {status && (
                      <div role="img" aria-label={resourceLabel} className="mr-3">
                        <StateIndicator
                          className="mr-0"
                          value={s.purpose === 'Image' ? status === 'In use' : status}
                          tooltip={resourceLabel}
                        />
                      </div>
                    )}
                    {link ? (
                      <Link className="table-link" to={link}>
                        {s.name}
                      </Link>
                    ) : (
                      s.name
                    )}
                  </div>
                </TableCell>
                <TableCell>{purposes[s.purpose]?.[1] ?? s.purpose}</TableCell>
                <TableCell>
                  {recurrence(s)}
                  {s.cron && <div className="font-mono text-xs text-muted-foreground">{s.cron}</div>}
                  {s.systemManaged && (
                    <div>
                      <Badge variant="outline" className="bg-muted/20 text-muted-foreground font-normal">
                        System managed
                      </Badge>
                    </div>
                  )}
                </TableCell>
                <TableCell>{s.timeZone ?? '—'}</TableCell>
                <TableCell>
                  <Timestamp value={s.nextAt} formatDateTime={formatDateTime} />
                </TableCell>
                <TableCell>
                  <div className="flex flex-wrap items-center gap-1">
                    <StateBadge value={s.needsAttention ? 'Warning' : state} label={state} />
                    {s.activeRunStatus && !s.latestRun && (
                      <StateBadge
                        kind="run"
                        value={s.activeRunStatus}
                        label={s.activeRunStatus === 'ApplyingRetention' ? 'Applying retention' : s.activeRunStatus}
                      />
                    )}
                  </div>
                  {s.reason && s.enabled && (
                    <div className="max-w-64 whitespace-normal text-xs text-muted-foreground">{s.reason}</div>
                  )}
                </TableCell>
                <TableCell>
                  {s.latestRun && link && (s.purpose === 'Script' || s.purpose === 'Backup') ? (
                    <div className="flex flex-wrap items-center gap-2">
                      <StateBadge value={s.latestRun.status} kind="run" />
                      <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
                        Queued <Timestamp value={s.latestRun.queuedAt} formatDateTime={formatDateTime} />
                      </span>
                      <Link
                        className="table-link"
                        aria-label={`View scheduled run for ${s.name}`}
                        to={`${link}?runId=${encodeURIComponent(s.latestRun.id)}#runs`}>
                        View run
                      </Link>
                    </div>
                  ) : (
                    '—'
                  )}
                </TableCell>
              </TableRow>
            );
          })}
        {open && !schedules.length && (
          <TableRow>
            <TableCell colSpan={7} className="py-4 text-center text-muted-foreground">
              {query.isError
                ? 'Schedules unavailable.'
                : query.isPending
                  ? 'Loading schedules…'
                  : 'No schedules on this page.'}
            </TableCell>
          </TableRow>
        )}
        {open && totalPages > 1 && (
          <TableRow>
            <TableCell colSpan={7}>
              <PaginationControls
                ariaLabel={`${label} schedule pages`}
                currentPage={page}
                totalPages={totalPages}
                onPageChange={(next) => {
                  if (next >= 1 && next <= totalPages) setPage(next);
                }}
              />
            </TableCell>
          </TableRow>
        )}
      </TableBody>
    </>
  );
}
