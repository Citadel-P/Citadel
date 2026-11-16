import { useState } from 'react';
import { ChevronRight } from 'lucide-react';
import type { JobStatusView, JobsSnapshotView } from '@/api/generated/api.types';
import { StateBadge } from '@/components/custom/state-badge';
import { StateIndicator } from '@/components/custom/state-indicator';
import { Button } from '@/components/ui/button';
import { TableBody, TableCell, TableRow } from '@/components/ui/table';
import type { DateTimeFormatter } from '@/lib/date-time';
import { type JobGroup, stateLabel } from './presentation';
import { JobRow } from './row';

export function JobGroupRows({
  group,
  visible,
  filtered,
  formatDateTime,
  snapshot,
  isLive,
}: {
  snapshot: JobsSnapshotView;
  isLive: boolean;
  group: JobGroup;
  visible: JobStatusView[];
  filtered: boolean;
  formatDateTime: DateTimeFormatter;
}) {
  const [expanded, setExpanded] = useState<boolean>();
  const open = expanded ?? filtered;
  const id = `job-group-${group.name.replaceAll(' ', '-')}`;
  return (
    <>
      <TableBody className="border-b border-border">
        <TableRow>
          <TableCell colSpan={6} className="whitespace-normal p-0">
            <Button
              variant="ghost"
              size="sm"
              className="h-auto min-h-(--table-row-height) w-full flex-wrap justify-start gap-x-2 rounded-none px-3 py-(--table-cell-padding-y) text-left font-normal whitespace-normal hover:bg-transparent dark:hover:bg-transparent"
              aria-label={`${group.name} jobs`}
              aria-expanded={open}
              aria-controls={id}
              onClick={() => setExpanded(!open)}>
              <ChevronRight aria-hidden="true" className={`size-4 shrink-0 ${open ? 'rotate-90' : ''}`} />
              <StateIndicator
                kind="job"
                value={group.state}
                tooltip={`${stateLabel(group.state)} · ${group.summary}`}
              />
              <span>{group.name}</span>
              <span className="text-xs font-normal text-muted-foreground tabular-nums">
                {visible.length === group.jobs.length ? group.jobs.length : `${visible.length}/${group.jobs.length}`}{' '}
                {group.jobs.length === 1 ? 'job' : 'jobs'}
              </span>
              <span className="ml-auto hidden text-xs font-normal text-muted-foreground sm:inline">
                {group.summary}
              </span>
              <StateBadge kind="job" value={group.state} label={stateLabel(group.state)} className="ml-auto sm:ml-2" />
            </Button>
            <p className="px-3 pb-2 text-xs text-muted-foreground sm:hidden">
              {group.summary}
              {visible.length !== group.jobs.length && <span> · {visible.length} matching</span>}
            </p>
          </TableCell>
        </TableRow>
      </TableBody>
      <TableBody id={id} hidden={!open}>
        {open &&
          visible.map((job) => (
            <JobRow key={job.id} job={job} snapshot={snapshot} isLive={isLive} formatDateTime={formatDateTime} />
          ))}
      </TableBody>
    </>
  );
}
