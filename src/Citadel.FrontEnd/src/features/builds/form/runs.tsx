import { ActorType, BuildProjectView, BuildRunLogEntry, BuildRunStatus, BuildRunView } from '@/api/generated/api.types';
import { LogViewer, type LogEntry } from '@/components/custom/common';
import { RunStatusBadge } from '@/components/custom/run-status-badge';
import SortableCell from '@/components/custom/sortable-cell';
import { StateIndicator } from '@/components/custom/state-indicator';
import { TimestampCell } from '@/components/custom/timestamp-cell';
import { Button } from '@/components/ui/button';
import { DataTable } from '@/components/ui/data-table';
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from '@/components/ui/sheet';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { parseCitadelDate } from '@/lib/date-time';
import { useMutate, useRead } from '@/lib/hooks';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { ColumnDef } from '@tanstack/react-table';
import { useQueryClient } from '@tanstack/react-query';
import { HubConnection } from '@microsoft/signalr';
import {
  Ban,
  Clock,
  FileCode2,
  FileText,
  Fingerprint,
  Folder,
  GitBranch,
  GitCommitHorizontal,
  Package,
  Server,
  Settings,
  SquareTerminal,
  type LucideIcon,
  User,
} from 'lucide-react';
import { type ReactNode, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { toast } from 'sonner';
import { formatBuildRunLogViewerEntry, mergeBuildRunLogEntries } from '../build-run-logs';
import { isActiveBuildRun, isTerminalBuildRunStatus, pickMostAdvancedBuildRun } from '../build-run-state';
import { useBuildRunQuery } from '../hooks/useBuildRunQuery';

export function BuildRunsTab({ resource }: { resource: BuildProjectView }) {
  const [logRunId, setLogRunId] = useState<string | undefined>();
  const { runId: requestedRunId, clearRunId } = useBuildRunQuery();
  const queryClient = useQueryClient();
  const cancelRun = useMutate('cancelBuildRun');
  const formatDateTime = useProfileDateTimeFormatter();
  const readArgs = useMemo(() => ({ query: { projectId: resource.id, limit: 50 } }), [resource.id]);
  const { data, isLoading } = useRead('listBuildRuns', readArgs);
  const [runs, setRuns] = useState<BuildRunView[] | undefined>();
  const [liveLogState, setLiveLogState] = useState<{ runId?: string; entries: BuildRunLogEntry[] }>({ entries: [] });
  const lastFetchedRef = useRef<BuildRunView[]>([]);
  const consumedRunIdRef = useRef<string | undefined>(undefined);
  const selectedRunArgs = useMemo(() => ({ id: logRunId ?? '' }), [logRunId]);
  const selectedRunQuery = useRead('getBuildRun', selectedRunArgs, {
    enabled: Boolean(logRunId),
  });

  useEffect(() => {
    if (!requestedRunId) {
      consumedRunIdRef.current = undefined;
      return;
    }

    if (consumedRunIdRef.current === requestedRunId) return;

    consumedRunIdRef.current = requestedRunId;
    setLogRunId(requestedRunId);
  }, [requestedRunId]);

  useEffect(() => {
    if (!data) return;

    const next = data.data.runs;
    if (next !== lastFetchedRef.current) {
      lastFetchedRef.current = next;
      setRuns(next);
    }
  }, [data]);

  const visibleRuns = useMemo(() => runs ?? [], [runs]);
  const selectedRunFromList = useMemo(() => visibleRuns.find((run) => run.id === logRunId), [logRunId, visibleRuns]);
  const selectedRunFromQuery =
    selectedRunQuery.data?.data.buildProjectId === resource.id ? selectedRunQuery.data.data : undefined;
  const selectedRun = useMemo(
    () => pickMostAdvancedBuildRun(selectedRunFromList, selectedRunFromQuery),
    [selectedRunFromList, selectedRunFromQuery],
  );
  const displayedRuns = useMemo(() => mergeSelectedRun(visibleRuns, selectedRun), [selectedRun, visibleRuns]);
  const logs = useRead('getBuildRunLogs', { id: logRunId ?? '' }, {
    enabled: Boolean(logRunId),
  });
  const logEntries = useMemo(() => {
    const liveLogs = liveLogState.runId === logRunId ? liveLogState.entries : [];
    return mergeBuildRunLogEntries(logs.data?.data.logs ?? [], liveLogs);
  }, [liveLogState, logRunId, logs.data?.data.logs]);
  const logViewerEntries = useMemo(() => logEntries.map(formatBuildRunLogViewerEntry), [logEntries]);

  const upsertRun = useCallback((run: BuildRunView, action: string) => {
    setRuns((prev) => {
      const current = prev ?? [];
      if (action === 'delete') {
        return current.filter((item) => item.id !== run.id);
      }

      const index = current.findIndex((item) => item.id === run.id);
      if (index === -1) {
        return sortRuns([run, ...current]);
      }

      const merged = pickMostAdvancedBuildRun(current[index], run);
      if (!merged || merged === current[index]) return prev;

      const updated = [...current];
      updated[index] = merged;
      return sortRuns(updated);
    });
  }, []);

  const handleBuildRunInfoUpdated = useCallback(
    (run: BuildRunView, action: string) => {
      if (run.buildProjectId !== resource.id) return;

      upsertRun(run, action);

      if (logRunId === run.id) {
        queryClient.setQueryData(['getBuildRun', { id: run.id }], { data: run });
        queryClient.invalidateQueries({ queryKey: ['getBuildRunLogs'] });
      }
    },
    [logRunId, queryClient, resource.id, upsertRun],
  );

  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.on('BuildRunInfoUpdated', handleBuildRunInfoUpdated);
    },
    [handleBuildRunInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.off('BuildRunInfoUpdated', handleBuildRunInfoUpdated);
    },
    [handleBuildRunInfoUpdated],
  );

  const handleBuildRunLogsAppended = useCallback(
    (runId: string, entries: BuildRunLogEntry[]) => {
      if (runId !== logRunId || entries.length === 0) return;

      setLiveLogState((prev) => ({
        runId,
        entries: mergeBuildRunLogEntries(prev.runId === runId ? prev.entries : [], entries),
      }));
    },
    [logRunId],
  );

  const setupLogEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.on('BuildRunLogsAppended', handleBuildRunLogsAppended);
    },
    [handleBuildRunLogsAppended],
  );

  const removeLogEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.off('BuildRunLogsAppended', handleBuildRunLogsAppended);
    },
    [handleBuildRunLogsAppended],
  );

  const handleRunGroupJoined = useCallback(() => {
    if (!logRunId) return;

    queryClient.invalidateQueries({ queryKey: ['getBuildRun', { id: logRunId }] });
    queryClient.invalidateQueries({ queryKey: ['getBuildRunLogs', { id: logRunId }] });
    queryClient.invalidateQueries({ queryKey: ['listBuildRuns', readArgs] });
  }, [logRunId, queryClient, readArgs]);

  useSignalRGroup({
    groupName: `build-runs:${resource.id}`,
    setupEventListeners,
    removeEventListeners,
  });

  useSignalRGroup({
    groupName: logRunId ? `build-run:${logRunId}` : undefined,
    setupEventListeners: setupLogEventListeners,
    removeEventListeners: removeLogEventListeners,
    onJoinedGroup: handleRunGroupJoined,
    skip: !logRunId,
  });

  const handleCancel = useCallback(
    async (run: BuildRunView) => {
      try {
        await cancelRun.mutateAsync({ id: run.id } as any);
        await queryClient.invalidateQueries({ queryKey: ['listBuildRuns'] });
        await queryClient.invalidateQueries({ queryKey: ['getBuildProject', { id: resource.id }] });
        toast.success('Build cancellation requested');
      } catch {
        /**Nope */
      }
    },
    [cancelRun, queryClient, resource.id],
  );

  const columns = useMemo(
    () => runColumns(setLogRunId, handleCancel, cancelRun.isPending, formatDateTime),
    [cancelRun.isPending, formatDateTime, handleCancel],
  );

  return (
    <div className="flex flex-col gap-4">
      <DataTable columns={columns} data={displayedRuns} isLoading={isLoading} />

      <BuildRunLogsSheet
        open={Boolean(logRunId)}
        run={selectedRun}
        logs={logViewerEntries}
        logEntryCount={logEntries.length}
        isLoading={logs.isLoading}
        cancelPending={cancelRun.isPending}
        formatDateTime={formatDateTime}
        onCancel={handleCancel}
        onOpenChange={(open) => {
          if (!open) {
            consumedRunIdRef.current = undefined;
            setLogRunId(undefined);
            clearRunId();
          }
        }}
      />
    </div>
  );
}

function sortRuns(runs: BuildRunView[]) {
  return [...runs].sort((a, b) => String(b.queuedAt).localeCompare(String(a.queuedAt)));
}

function mergeSelectedRun(runs: BuildRunView[], selectedRun?: BuildRunView) {
  if (!selectedRun) return runs;

  const index = runs.findIndex((run) => run.id === selectedRun.id);
  if (index === -1) return sortRuns([selectedRun, ...runs]);

  const merged = pickMostAdvancedBuildRun(runs[index], selectedRun);
  if (!merged || merged === runs[index]) return runs;

  const updated = [...runs];
  updated[index] = merged;
  return sortRuns(updated);
}

const runColumns = (
  onSelectLog: (id: string) => void,
  onCancel: (run: BuildRunView) => void,
  cancelPending: boolean,
  formatDateTime: ReturnType<typeof useProfileDateTimeFormatter>,
): ColumnDef<BuildRunView>[] => [
  {
    accessorKey: 'trigger',
    header: ({ column }) => <SortableCell cellName="Trigger" column={column} />,
    cell: ({ row }) => <span className="text-sm">{row.original.trigger}</span>,
    sortingFn: (rowA, rowB) => rowA.original.trigger.localeCompare(rowB.original.trigger),
  },
  {
    accessorKey: 'status',
    header: ({ column }) => <SortableCell cellName="Status" column={column} />,
    cell: ({ row }) => <RunStatusBadge status={row.original.status} />,
    sortingFn: (rowA, rowB) => rowA.original.status.localeCompare(rowB.original.status),
  },
  {
    accessorKey: 'resolvedCommitSha',
    header: ({ column }) => <SortableCell cellName="Commit" column={column} />,
    cell: ({ row }) => <span className="text-sm">{row.original.resolvedCommitSha?.slice(0, 12) ?? '-'}</span>,
  },
  {
    accessorKey: 'imageReferences',
    header: ({ column }) => <SortableCell cellName="Images" column={column} />,
    cell: ({ row }) => (
      <span className="block max-w-96 truncate text-sm" title={row.original.imageReferences.join(', ')}>
        {row.original.imageReferences.join(', ') || '-'}
      </span>
    ),
  },
  {
    accessorKey: 'queuedAt',
    header: ({ column }) => <SortableCell cellName="Queued" column={column} />,
    cell: ({ row }) => <TimestampCell value={row.original.queuedAt} formatDateTime={formatDateTime} />,
    sortingFn: (rowA, rowB) => String(rowA.original.queuedAt).localeCompare(String(rowB.original.queuedAt)),
  },
  {
    accessorKey: 'exitCode',
    header: ({ column }) => <SortableCell cellName="Exit" column={column} />,
    cell: ({ row }) => <span className="text-sm">{row.original.exitCode ?? '-'}</span>,
    sortingFn: (rowA, rowB) => Number(rowA.original.exitCode ?? -1) - Number(rowB.original.exitCode ?? -1),
  },
  {
    id: 'actions',
    cell: ({ row }) => {
      const cancellable = isActiveBuildRun(row.original);
      return (
        <div className="flex justify-end gap-1">
          <Button type="button" size="icon-sm" variant="ghost" onClick={() => onSelectLog(row.original.id)} title="View logs">
            <FileText className="size-3.5" />
          </Button>
          {cancellable && (
            <Button
              type="button"
              size="icon-sm"
              variant="ghost"
              disabled={cancelPending}
              onClick={() => onCancel(row.original)}
              title="Cancel build">
              <Ban className="size-3.5" />
            </Button>
          )}
        </div>
      );
    },
  },
];

function BuildRunLogsSheet({
  open,
  run,
  logs,
  logEntryCount,
  isLoading,
  cancelPending,
  formatDateTime,
  onCancel,
  onOpenChange,
}: {
  open: boolean;
  run?: BuildRunView;
  logs: LogEntry[];
  logEntryCount: number;
  isLoading: boolean;
  cancelPending: boolean;
  formatDateTime: ReturnType<typeof useProfileDateTimeFormatter>;
  onCancel: (run: BuildRunView) => void;
  onOpenChange: (open: boolean) => void;
}) {
  const active = run ? isActiveBuildRun(run) : false;
  const actorQuery = useRead('getActor', { id: run?.triggeredByActorId ?? '' }, { enabled: Boolean(run?.triggeredByActorId) });
  const actor = actorQuery.data?.data;
  const description = run
    ? `${run.trigger} run on ${run.branch}`
    : isLoading
      ? 'Loading build logs...'
      : 'Build logs';

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent
        onOpenAutoFocus={(event) => event.preventDefault()}
        side="bottom"
        className="mx-auto h-auto max-h-[calc(100dvh-1rem)] w-300 max-w-[100vw] gap-0 rounded-t-md">
        <div className="flex min-h-0 flex-1 flex-col">
          <SheetHeader className="border-b pr-12">
            <div className="flex flex-wrap items-start justify-between gap-4">
              <div className="min-w-0">
                <SheetTitle className="truncate">{run ? `${run.projectNameSnapshot} run` : 'Build run'}</SheetTitle>
                <SheetDescription asChild>
                  <div className="mt-1 flex flex-wrap items-center gap-x-4 gap-y-1">
                    <span>{description}</span>
                    {run ? (
                      <>
                        <span className="inline-flex items-center gap-1.5">
                          <Clock className="size-3.5" />
                          {formatDateTime(run.queuedAt)}
                        </span>
                        <span>{formatDuration(run)}</span>
                        <span>{logEntryCount} log entries</span>
                      </>
                    ) : null}
                  </div>
                </SheetDescription>
              </div>
              {run ? (
                <div className="flex shrink-0 items-center gap-3">
                  <span className="inline-flex items-center gap-2 text-sm">
                    <StateIndicator value={run.status} isProcessing={active} kind="buildRun" />
                    {run.status}
                  </span>
                  {active ? (
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      disabled={cancelPending}
                      onClick={() => onCancel(run)}>
                      <Ban className="size-4" />
                      Cancel
                    </Button>
                  ) : null}
                </div>
              ) : null}
            </div>
          </SheetHeader>

          <div className="flex min-h-0 flex-1 flex-col p-4">
            {run ? (
              <div className="mb-3 ml-2 grid gap-x-6 gap-y-2 text-sm text-muted-foreground sm:grid-cols-2 lg:grid-cols-3">
                <MetadataRow
                  icon={actor?.type === ActorType.System ? Settings : User}
                  label="Actor"
                  value={actor ? `${actor.name} (${actor.type})` : actorQuery.isLoading ? 'Loading...' : shortId(run.triggeredByActorId)}
                  title={run.triggeredByActorId}
                />
                <MetadataRow icon={GitBranch} label="Repository" value={run.gitRepositoryNameSnapshot} />
                <MetadataRow icon={GitCommitHorizontal} label="Commit" value={run.resolvedCommitSha?.slice(0, 12) ?? '-'} monospace />
                <MetadataRow icon={Package} label="Image" value={run.imageRepository} />
                <MetadataRow icon={Server} label="Platform" value={run.platformSnapshot.name} />
                <MetadataRow icon={FileCode2} label="Dockerfile" value={run.dockerfilePath} monospace />
                <MetadataRow icon={Folder} label="Context" value={run.contextPath} monospace />
                <MetadataRow icon={SquareTerminal} label="Exit code" value={run.exitCode ?? '-'} />
                <MetadataRow icon={Fingerprint} label="Digest" value={run.imageDigest?.slice(0, 20) ?? '-'} monospace />
              </div>
            ) : null}

            <LogViewer
              logs={isLoading ? [] : logs}
              autoScroll
              allowWrap
              timeStamps
              className="h-[min(48dvh,440px)] max-h-none bg-background"
            />
          </div>
        </div>
      </SheetContent>
    </Sheet>
  );
}

function MetadataRow({
  icon: Icon,
  label,
  value,
  monospace,
  title,
}: {
  icon: LucideIcon;
  label: string;
  value: ReactNode;
  monospace?: boolean;
  title?: string;
}) {
  return (
    <div className="flex min-w-0 items-start gap-2" title={title}>
      <Icon className="mt-0.5 size-3.5 shrink-0 text-muted-foreground" />
      <div className="min-w-0">
        <dt className="text-xs text-muted-foreground">{label}</dt>
        <dd className={monospace ? 'mt-0.5 break-all font-mono text-xs text-foreground' : 'mt-0.5 break-words text-sm text-foreground'}>
          {value}
        </dd>
      </div>
    </div>
  );
}

function formatDuration(run: BuildRunView) {
  const startedAt = parseCitadelDate(run.startedAt);
  if (!startedAt) return '-';

  const completedAt = parseCitadelDate(run.completedAt) ?? (isActiveBuildRun(run) ? new Date() : null);
  if (!completedAt) return '-';

  const totalSeconds = Math.max(0, Math.round((completedAt.getTime() - startedAt.getTime()) / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return minutes > 0 ? `${minutes}m ${seconds}s` : `${seconds}s`;
}

function shortId(value: string) {
  return value ? `${value.slice(0, 8)}...` : '-';
}

export function isTerminalBuildStatus(status: BuildRunStatus) {
  return isTerminalBuildRunStatus(status);
}
