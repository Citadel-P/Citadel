import { PageError } from '@/components/custom/page-error';
import { useState } from 'react';
import { SearchField } from '@/components/custom/search-field';
import { CircleDot, Layers } from 'lucide-react';
import { ResourceMultiFilter } from '@/components/custom/resource-filter';
import { instanceResourceOption } from '../resource-options';
import { jobStateOption } from './filters';
import { Button } from '@/components/ui/button';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { JobStateView } from '@/api/generated/api.types';
import { displayState, groupJobs } from './presentation';
import { JobGroupRows } from './group';
import { useInstanceJobs } from './useInstanceJobs';
import { JobsSummary } from './summary';
import { describeJob } from './descriptions';

export function InstanceJobs() {
  const result = useInstanceJobs();
  const [filter, setFilter] = useState('');
  const [states, setStates] = useState<string[]>([]);
  const formatDateTime = useProfileDateTimeFormatter();
  const snapshot = result.data;
  const jobs = snapshot?.jobs ?? [];
  const [resources, setResources] = useState<string[]>([]);
  const groups = groupJobs(jobs);
  const search = filter.trim().toLowerCase();
  const filtered = states.length > 0 || resources.length > 0 || !!search;
  const visible = groups
    .filter((group) => !resources.length || resources.includes(group.name))
    .map((group) => ({
      group,
      jobs: group.jobs.filter(
        (job) =>
          (!states.length || states.includes(displayState(job))) &&
          `${group.name} ${job.name} ${job.id} ${describeJob(job).name}`.toLowerCase().includes(search),
      ),
    }))
    .filter((group) => group.jobs.length > 0);
  if (result.denied) return <PageError status={403} message="Administrator access is required to view jobs." />;
  return (
    <section className="space-y-4" aria-label="Background jobs">
      {snapshot && !result.isLive && (
        <p role="status" className="text-sm text-muted-foreground">
          Showing last known state. Live observations are unavailable.
        </p>
      )}
      {snapshot && <JobsSummary jobs={jobs} />}
      <div
        role="region"
        aria-label="Job filters"
        className="flex min-w-0 flex-wrap items-center gap-2 rounded-lg border bg-card p-3 shadow-xs">
        <SearchField placeholder="Search jobs…" className="mb-0 w-full sm:w-72" onSearch={setFilter} />
        <div className="flex min-w-0 flex-wrap items-center gap-2 sm:ml-auto">
          <ResourceMultiFilter
            label="Resource"
            icon={Layers}
            selected={resources}
            onChange={setResources}
            options={groups.map((group) => instanceResourceOption(group.name))}
          />
          <ResourceMultiFilter
            label="State"
            icon={CircleDot}
            selected={states}
            onChange={setStates}
            options={[...Object.values(JobStateView), 'failed' as const].map(jobStateOption)}
          />
        </div>
      </div>
      {result.isError && (
        <div role="alert" className="flex items-center gap-3 text-sm">
          Could not load the current jobs snapshot.
          <Button variant="outline" size="sm" onClick={() => void result.retry()}>
            Retry
          </Button>
        </div>
      )}
      {result.isLoading && (
        <p role="status" className="py-8 text-center text-muted-foreground">
          Loading jobs…
        </p>
      )}
      {snapshot && (
        <Table aria-label="Background jobs">
          <TableHeader>
            <TableRow>
              <TableHead>Job</TableHead>
              <TableHead>State</TableHead>
              <TableHead className="hidden lg:table-cell">Latest result</TableHead>
              <TableHead className="hidden xl:table-cell">Last started</TableHead>
              <TableHead className="hidden md:table-cell">Elapsed time</TableHead>
              <TableHead className="hidden lg:table-cell">Next check</TableHead>
            </TableRow>
          </TableHeader>
          {visible.map(({ group, jobs: matching }) => (
            <JobGroupRows
              key={`${group.name}-${states.join(',')}-${resources.join(',')}-${search}`}
              snapshot={snapshot}
              isLive={result.isLive}
              group={group}
              visible={matching}
              filtered={filtered}
              formatDateTime={formatDateTime}
            />
          ))}
          {!visible.length && (
            <TableBody>
              <TableRow>
                <TableCell colSpan={6} className="py-8 text-center text-muted-foreground">
                  {jobs.length ? 'No jobs match your filters.' : 'No jobs registered.'}
                </TableCell>
              </TableRow>
            </TableBody>
          )}
        </Table>
      )}
      <p className="text-xs text-muted-foreground">
        Counters show work since this Core started and reset on restart. Saved history from previous sessions remains
        available for up to seven days. Durations include I/O waits and do not measure CPU usage. Timings are
        unavailable until a measured check completes.
      </p>
    </section>
  );
}
