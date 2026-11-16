import { useEffect, useRef, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import type { JobsSnapshotView, JobStatusView } from '@/api/generated/api.types';
import { useRead } from '@/lib/hooks';
import { canShowCachedResource } from '@/lib/request-error';
import { resourceQueryKey } from '@/api/dispatch';
import { useCopyToClipboard } from '@/hooks/useCopyToClipboard';
import { Button } from '@/components/ui/button';
import { History, Copy } from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogTrigger,
} from '@/components/ui/dialog';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import type { DateTimeFormatter } from '@/lib/date-time';
import { diagnostics } from './diagnostics';
import { describeJob } from './descriptions';
import { JobTrends } from './trends';
import { JobProblemTarget } from './problem';
import { useInterval } from '@/hooks/useInterval';

type HistoryProps = {
  job: JobStatusView;
  snapshot: JobsSnapshotView;
  isLive: boolean;
  formatDateTime: DateTimeFormatter;
};

export function JobHistoryDialog(props: HistoryProps) {
  const [open, setOpen] = useState(false);
  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button variant="outline" size="sm">
          <History aria-hidden="true" />
          History and work trends
        </Button>
      </DialogTrigger>
      <DialogContent className="flex max-h-[calc(100dvh-2rem)] flex-col gap-0 overflow-hidden p-0 sm:max-w-3xl">
        <DialogHeader className="shrink-0 border-b px-4 py-4 pr-12 text-left sm:px-6 sm:pr-12">
          <DialogTitle className="text-base">{describeJob(props.job).name}</DialogTitle>
          <DialogDescription className="text-xs">
            History and work trends · retained for up to seven days
          </DialogDescription>
        </DialogHeader>
        <div className="min-h-0 flex-1 overflow-y-auto p-4 sm:p-6">{open && <JobsHistory {...props} />}</div>
      </DialogContent>
    </Dialog>
  );
}

function JobsHistory({ job, snapshot, isLive, formatDateTime }: HistoryProps) {
  const collection = job.id === 'local-container-stats';
  const [now, setNow] = useState(() => Date.now());
  // Age the displayed observation locally; this timer never fetches data.
  useInterval(() => setNow(Date.now()), 15_000);
  const [hours, setHours] = useState('24');
  const [copied, copy] = useCopyToClipboard(3000);
  const [copyFailed, setCopyFailed] = useState(false);
  const query = useRead(
    'getInstanceJobHistory',
    { query: { job: job.id, hours: Number(hours) } },
    {
      staleTime: Infinity,
      retry: false,
      refetchOnMount: 'always',
      refetchOnWindowFocus: false,
      refetchOnReconnect: false,
      meta: { suppressErrorToast: true },
    },
  );
  const history = canShowCachedResource(query.error) ? query.data?.data : undefined;
  const client = useQueryClient();
  const last = useRef(`${snapshot.bootId}:${snapshot.historyRevision}`);
  useEffect(() => {
    const revision = `${snapshot.bootId}:${snapshot.historyRevision}`;
    if (last.current === revision) return;
    last.current = revision;
    void client.invalidateQueries({ queryKey: [resourceQueryKey('getInstanceJobHistory')[0]] });
  }, [snapshot.bootId, snapshot.historyRevision, client]);
  const jobs = new Map<string, Pick<JobsSnapshotView['jobs'][number], 'id' | 'name'>>(
    snapshot.jobs.map((j) => [j.id, j]),
  );
  for (const session of history?.sessions ?? [])
    for (const j of session.snapshot.jobs) if (!jobs.has(j.id)) jobs.set(j.id, j);
  for (const { event } of history?.events ?? [])
    if (event.jobId && !jobs.has(event.jobId)) jobs.set(event.jobId, { id: event.jobId, name: event.jobId });
  const label = (id: string) => (jobs.has(id) ? describeJob(jobs.get(id)!).name : id);
  const events = (history?.events ?? []).map(({ bootId, event }) => ({
    key: `${bootId}-${event.sequence}`,
    at: event.at,
    text:
      event.kind === 'coreStarted'
        ? 'Core started'
        : event.kind === 'observationsLost'
          ? 'Some observations were lost before they could be saved'
          : `${label(event.jobId)} · ${{ failed: 'Attempt failed', problem: 'Problem reported', recovered: 'A subsequent check succeeded', stopped: 'Worker stopped' }[event.kind] ?? 'State changed'}`,
    failed: event.kind === 'failed' || event.kind === 'problem',
    problem: event.problem,
  }));
  for (const s of history?.sessions ?? []) {
    if (s.stoppedAt != null)
      events.push({
        key: `${s.snapshot.bootId}-stop`,
        at: s.stoppedAt,
        text: 'Diagnostic reporting stopped during shutdown',
        failed: false,
        problem: null,
      });
  }
  events.sort((a, b) => b.at - a.at);
  return (
    <section className="space-y-4" aria-label="Job history">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <Select value={hours} onValueChange={setHours}>
          <SelectTrigger aria-label="History range" className="w-40">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {[
              ['1', 'Last hour'],
              ['6', 'Last 6 hours'],
              ['24', 'Last 24 hours'],
              ['168', 'Last 7 days'],
            ].map(([value, label]) => (
              <SelectItem key={value} value={value}>
                {label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Button
          variant="outline"
          size="sm"
          onClick={() =>
            void copy(
              diagnostics({ ...snapshot, jobs: snapshot.jobs.filter((j) => j.id === job.id) }, history, isLive),
            ).then((ok) => setCopyFailed(!ok))
          }>
          <Copy aria-hidden="true" />
          {copied ? 'Copied' : 'Copy diagnostics'}
        </Button>
      </div>
      {copyFailed && <p role="alert">Could not copy diagnostics. Clipboard access is unavailable.</p>}
      {query.isPending && <p role="status">Loading retained observations…</p>}
      {query.isError && (
        <p role="alert">History is unavailable. Current worker observations are independent of diagnostic storage.</p>
      )}
      {history && (
        <>
          {!isLive && (
            <p className="text-sm text-muted-foreground">Live updates unavailable · showing retained observations.</p>
          )}
          {history.truncated && (
            <p className="text-sm text-muted-foreground">
              Showing the most recent records within the response limit. Choose a shorter range for more detail.
            </p>
          )}
          <Tabs defaultValue="events">
            <TabsList aria-label="Job history views" className="w-full">
              <TabsTrigger value="events">Events</TabsTrigger>
              <TabsTrigger value="trends">{collection ? 'Collection time' : 'Work trends'}</TabsTrigger>
              <TabsTrigger value="sessions">Core sessions</TabsTrigger>
            </TabsList>
            <TabsContent value="trends" className="pt-4">
              <JobTrends
                buckets={history.buckets}
                sessions={history.sessions}
                formatDateTime={formatDateTime}
                collection={collection}
              />
            </TabsContent>
            <TabsContent
              value="sessions"
              className="pt-4 [&>[data-slot=table-container]]:max-h-80 [&>[data-slot=table-container]]:overflow-y-auto">
              <Table>
                <TableHeader className="sticky top-0 z-10 bg-card">
                  <TableRow>
                    <TableHead>Core / session</TableHead>
                    <TableHead>Version</TableHead>
                    <TableHead>Reporting</TableHead>
                    <TableHead>Last saved observation</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {history.sessions.map((s) => (
                    <TableRow key={s.snapshot.bootId}>
                      <TableCell>
                        <span title={s.coreId}>{s.coreId.slice(0, 8)}</span> /{' '}
                        <span title={s.snapshot.bootId}>{s.snapshot.bootId.slice(0, 8)}</span>
                        {s.snapshot.bootId === snapshot.bootId && ' · This session'}
                      </TableCell>
                      <TableCell>{s.version}</TableCell>
                      <TableCell>
                        {s.stoppedAt
                          ? 'Stopped'
                          : !isLive || Math.max(history.observedAt, snapshot.observedAt, now) - s.heartbeatAt > 90_000
                            ? 'Unknown · not reporting'
                            : 'Recently reported'}
                        {s.lostEvents > 0 && ` · ${s.lostEvents} lost observations`}
                      </TableCell>
                      <TableCell>{formatDateTime(s.snapshot.observedAt)}</TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </TabsContent>
            <TabsContent value="events" className="pt-4">
              <div className="max-h-80 overflow-y-auto px-1 py-1">
                {events.length === 0 ? (
                  <p className="py-4 text-sm text-muted-foreground">
                    No recorded events in this range. Missing observations do not confirm success.
                  </p>
                ) : (
                  <ol aria-label="Job event timeline" className="ml-1 space-y-5 border-l pl-5">
                    {events.map((event) => (
                      <li key={event.key} className="relative text-sm">
                        <span
                          aria-hidden="true"
                          className={`absolute -left-[25px] top-1.5 size-2 rounded-full ring-4 ring-background ${event.failed ? 'bg-destructive' : 'bg-muted-foreground'}`}
                        />
                        <p className={event.failed ? 'text-destructive' : ''}>{event.text}</p>
                        <p className="text-xs text-muted-foreground">{formatDateTime(event.at)}</p>
                        {event.problem && (
                          <div className="mt-2 space-y-1">
                            <p>{event.problem.summary}</p>
                            <JobProblemTarget target={event.problem.target} />
                          </div>
                        )}
                      </li>
                    ))}
                  </ol>
                )}
              </div>
            </TabsContent>
          </Tabs>
          <p className="border-t pt-3 text-xs leading-relaxed text-muted-foreground">
            Events save every 30 seconds; work summaries every five minutes. History is best effort. Abrupt stops can
            lose buffered events and the latest work summary. Missing observations are not successful checks.
          </p>
        </>
      )}
    </section>
  );
}
