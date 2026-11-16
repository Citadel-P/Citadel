import { useState, type ReactNode } from 'react';
import { describeJob, latestResult } from './descriptions';
import { ChevronRight } from 'lucide-react';
import type { JobStatusView, JobsSnapshotView } from '@/api/generated/api.types';
import { StateBadge } from '@/components/custom/state-badge';
import { StateIndicator } from '@/components/custom/state-indicator';
import { Button } from '@/components/ui/button';
import { TableCell, TableRow } from '@/components/ui/table';
import { Timestamp } from '@/components/custom/timestamp';
import type { DateTimeFormatter } from '@/lib/date-time';
import { displayState, stateLabel } from './presentation';
import { JobHistoryDialog } from './history';
import { JobProblemTarget } from './problem';

const duration = (value?: number | null) =>
  value == null
    ? 'Unavailable'
    : value < 1000
      ? `${value.toLocaleString(undefined, { maximumFractionDigits: 2 })} ms`
      : `${(value / 1000).toLocaleString(undefined, { maximumFractionDigits: 2 })} s`;
const outcome = {
  completed: 'Completed · work count unavailable',
  productive: 'Completed with work',
  noWork: 'No work',
  budgetExhausted: 'Budget exhausted / work may remain',
  failed: 'Failed',
  cancelled: 'Cancelled',
  interrupted: 'Interrupted',
};
function Detail({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex flex-wrap justify-between gap-x-4 gap-y-1">
      <dt className="text-muted-foreground">{label}</dt>
      <dd>{children}</dd>
    </div>
  );
}

export function JobRow({
  job,
  snapshot,
  isLive,
  formatDateTime,
}: {
  job: JobStatusView;
  snapshot: JobsSnapshotView;
  isLive: boolean;
  formatDateTime: DateTimeFormatter;
}) {
  const [open, setOpen] = useState(false);
  const state = displayState(job);
  const time = (value?: number | null) =>
    value == null ? 'Unavailable' : <Timestamp value={value} formatDateTime={formatDateTime} />;
  const next =
    state === 'stopped' ||
    state === 'disabled' ||
    (state === 'failed' && job.nextCheck == null && job.waitingFor == null) ? (
      'Not scheduled'
    ) : job.state === 'running' ? (
      'After completion'
    ) : job.nextCheck != null ? (
      !isLive ? (
        'Live schedule unavailable'
      ) : job.nextCheck <= snapshot.observedAt ? (
        <span
          title={`Last reported deadline: ${formatDateTime(job.nextCheck, 'exact')}. Awaiting a newer worker observation; this does not establish a failure.`}>
          Due · awaiting update
        </span>
      ) : (
        time(job.nextCheck)
      )
    ) : (
      (job.waitingFor ?? 'Unavailable')
    );
  return (
    <>
      <TableRow>
        <TableCell className="whitespace-normal pl-8">
          <Button
            variant="ghost"
            size="sm"
            className="-ml-2 h-auto max-w-full justify-start py-1 text-left font-normal whitespace-normal hover:bg-transparent dark:hover:bg-transparent"
            aria-expanded={open}
            aria-controls={`job-${job.id}`}
            onClick={() => setOpen(!open)}>
            <ChevronRight aria-hidden="true" className={`size-4 shrink-0 ${open ? 'rotate-90' : ''}`} />
            <StateIndicator kind="job" value={state} tooltip={stateLabel(state)} />
            <span>{describeJob(job).name}</span>
          </Button>
          {job.activeOperations != null && (
            <p className="pl-6 text-xs text-muted-foreground">{job.activeOperations} active operations</p>
          )}
        </TableCell>
        <TableCell>
          <StateBadge kind="job" value={state} label={stateLabel(state)} />
        </TableCell>
        <TableCell className="hidden max-w-64 whitespace-normal lg:table-cell">
          {latestResult(job)}
          {job.lastProblem && (
            <Button variant="link" size="sm" className="block h-auto p-0 text-xs" onClick={() => setOpen(true)}>
              View last reported problem
            </Button>
          )}
        </TableCell>
        <TableCell className="hidden xl:table-cell">{time(job.lastStartedAt)}</TableCell>
        <TableCell className="hidden md:table-cell">
          {job.state === 'running' ? 'In progress' : duration(job.lastDurationMs)}
        </TableCell>
        <TableCell className="hidden max-w-56 whitespace-normal lg:table-cell">{next}</TableCell>
      </TableRow>
      {open && (
        <TableRow id={`job-${job.id}`}>
          <TableCell colSpan={6} className="whitespace-normal bg-muted/15 px-4 py-4 sm:pl-14 sm:pr-6">
            <div className="space-y-4 text-sm">
              <p className="max-w-3xl leading-relaxed text-muted-foreground">{describeJob(job).purpose}</p>
              <dl className="grid gap-4 sm:grid-cols-2">
                <div className="space-y-1">
                  <dt className="text-xs text-muted-foreground">Latest result</dt>
                  <dd>{latestResult(job)}</dd>
                </div>
                <div className="space-y-1">
                  <dt className="text-xs text-muted-foreground">Next check</dt>
                  <dd>{next}</dd>
                </div>
              </dl>
              {job.lastProblem && (
                <section aria-label="Last reported problem" className="space-y-2 rounded-md border p-3">
                  <p className="text-xs text-muted-foreground">Last reported problem · {time(job.lastProblem.at)}</p>
                  <p>{job.lastProblem.summary}</p>
                  <JobProblemTarget target={job.lastProblem.target} />
                  {job.lastProblem.target && (
                    <p className="text-xs text-muted-foreground">
                      Open the resource for its current status. A later scheduler check does not confirm recovery.
                    </p>
                  )}
                </section>
              )}
              <JobHistoryDialog job={job} snapshot={snapshot} isLive={isLive} formatDateTime={formatDateTime} />
            </div>
            <details className="mt-4 border-t pt-2">
              <summary className="cursor-pointer py-2 text-sm text-muted-foreground">Technical details</summary>
              <div className="grid gap-3 text-xs lg:grid-cols-3">
                <section className="space-y-3 rounded-md border bg-background p-3">
                  <h3 className="font-semibold">Schedule</h3>
                  <dl className="space-y-2">
                    <Detail label="Trigger">{job.trigger ?? 'Unavailable'}</Detail>
                    <Detail label="Budget">{duration(job.budgetMs)}</Detail>
                    <Detail label="Ownership">
                      {
                        {
                          processLocal: 'Process local',
                          coordinator: 'Coordinator',
                          targetSession: 'Target / session',
                        }[job.ownership]
                      }
                    </Detail>
                    <Detail label="Job ID">
                      <code>{job.id}</code>
                    </Detail>
                    <Detail label="Current start">{time(job.currentStartedAt)}</Detail>
                    <Detail label="Active iterations">{job.activeIterations}</Detail>
                    <Detail label="Last started">{time(job.lastStartedAt)}</Detail>
                  </dl>
                </section>
                <section className="space-y-3 rounded-md border bg-background p-3">
                  <h3 className="font-semibold">Last outcome</h3>
                  <p>{job.lastOutcome ? outcome[job.lastOutcome] : 'No measured iteration yet'}</p>
                  <dl className="space-y-2">
                    <Detail label="Completed">{time(job.lastCompletedAt)}</Detail>
                    <Detail label="Checks since Core started">{job.iterations}</Detail>
                    <Detail label="Productive / no work">
                      {job.productive} / {job.noWork}
                    </Detail>
                    <Detail label="Budget / work remains">{job.budgetExhaustions}</Detail>
                    <Detail label="Completed work since Core started">
                      {job.completedUnits == null
                        ? 'Unavailable'
                        : `${job.completedUnits.toLocaleString()} ${job.unit ?? ''}`}
                    </Detail>
                    <Detail label="Backlog">
                      {job.backlog == null
                        ? 'Unavailable'
                        : `${job.backlogIsLowerBound ? 'At least ' : ''}${job.backlog.toLocaleString()}`}
                    </Detail>
                    <Detail label="Last elapsed time">{duration(job.lastDurationMs)}</Detail>
                    <Detail label="Total / mean / max">
                      {duration(job.totalDurationMs)} /{' '}
                      {duration(job.iterations ? job.totalDurationMs / job.iterations : null)} /{' '}
                      {duration(job.iterations ? job.maxDurationMs : null)}
                    </Detail>
                  </dl>
                </section>
                <section className="space-y-3 rounded-md border bg-background p-3">
                  <h3 className="font-semibold">Observations</h3>
                  <dl className="space-y-2">
                    <Detail label="Last successful check">{time(job.lastSuccessAt)}</Detail>
                    <Detail label="Last productive progress">{time(job.lastProgressAt)}</Detail>
                    <Detail label="Failures since startup">{job.failures}</Detail>
                    <Detail label="Last failure">{time(job.lastFailureAt)}</Detail>
                  </dl>
                  {job.lastError && (
                    <p
                      className={`break-words ${state === 'failed' || state === 'backingOff' ? 'text-destructive' : 'text-muted-foreground'}`}>
                      Last failure: {job.lastError}
                    </p>
                  )}
                </section>
              </div>
            </details>
          </TableCell>
        </TableRow>
      )}
    </>
  );
}
