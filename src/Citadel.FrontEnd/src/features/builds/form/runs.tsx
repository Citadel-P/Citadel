import { BuildProjectView, BuildRunStatus, BuildRunView } from '@/api/generated/api.types';
import { ContentCard } from '@/components/custom/content-card';
import { LogViewer } from '@/components/custom/common';
import SortableCell from '@/components/custom/sortable-cell';
import { StateIndicator } from '@/components/custom/state-indicator';
import { TimestampCell } from '@/components/custom/timestamp-cell';
import { Button } from '@/components/ui/button';
import { DataTable } from '@/components/ui/data-table';
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from '@/components/ui/sheet';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { useMutate, useRead } from '@/lib/hooks';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { ColumnDef } from '@tanstack/react-table';
import { useQueryClient } from '@tanstack/react-query';
import { HubConnection } from '@microsoft/signalr';
import { Ban, FileText } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { toast } from 'sonner';
import { isActiveRun } from '../table';

export function BuildRunsTab({ resource }: { resource: BuildProjectView }) {
  const [logRunId, setLogRunId] = useState<string | undefined>();
  const queryClient = useQueryClient();
  const cancelRun = useMutate('cancelBuildRun');
  const formatDateTime = useProfileDateTimeFormatter();
  const readArgs = useMemo(() => ({ query: { projectId: resource.id, limit: 50 } }), [resource.id]);
  const { data, isLoading } = useRead('listBuildRuns', readArgs, {
    refetchInterval: (query) => {
      const runs = query.state.data?.data.runs ?? [];
      return runs.some(isActiveRun) || resource.currentRunId ? 3000 : false;
    },
  });
  const [runs, setRuns] = useState<BuildRunView[] | undefined>();
  const lastFetchedRef = useRef<BuildRunView[]>([]);

  useEffect(() => {
    if (!data) return;

    const next = data.data.runs;
    if (next !== lastFetchedRef.current) {
      lastFetchedRef.current = next;
      setRuns(next);
    }
  }, [data]);

  const visibleRuns = useMemo(() => runs ?? [], [runs]);
  const selectedRun = useMemo(() => visibleRuns.find((run) => run.id === logRunId), [logRunId, visibleRuns]);
  const logs = useRead('getBuildRunLogs', { id: logRunId ?? '' }, {
    enabled: Boolean(logRunId),
    refetchInterval: selectedRun && isActiveRun(selectedRun) ? 2000 : false,
  });

  const handleBuildRunInfoUpdated = useCallback(
    (run: BuildRunView, action: string) => {
      if (run.buildProjectId !== resource.id) return;

      setRuns((prev) => {
        const current = prev ?? [];
        if (action === 'delete') {
          return current.filter((item) => item.id !== run.id);
        }

        const index = current.findIndex((item) => item.id === run.id);
        if (index === -1) {
          return sortRuns([run, ...current]);
        }

        const updated = [...current];
        updated[index] = run;
        return sortRuns(updated);
      });

      if (logRunId === run.id) {
        queryClient.invalidateQueries({ queryKey: ['getBuildRunLogs'] });
      }
    },
    [logRunId, queryClient, resource.id],
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

  useSignalRGroup({
    groupName: `build-runs:${resource.id}`,
    setupEventListeners,
    removeEventListeners,
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
      <ContentCard>
        <DataTable columns={columns} data={visibleRuns} isLoading={isLoading} />
      </ContentCard>

      <BuildRunLogsSheet
        open={Boolean(logRunId)}
        run={selectedRun}
        logs={(logs.data?.data.logs ?? []).map((entry) => `[${entry.stream}] ${entry.message}`).join('\n')}
        isLoading={logs.isLoading}
        onOpenChange={(open) => {
          if (!open) setLogRunId(undefined);
        }}
      />
    </div>
  );
}

function sortRuns(runs: BuildRunView[]) {
  return [...runs].sort((a, b) => String(b.queuedAt).localeCompare(String(a.queuedAt)));
}

const runColumns = (
  onSelectLog: (id: string) => void,
  onCancel: (run: BuildRunView) => void,
  cancelPending: boolean,
  formatDateTime: ReturnType<typeof useProfileDateTimeFormatter>,
): ColumnDef<BuildRunView>[] => [
  {
    accessorKey: 'status',
    header: ({ column }) => <SortableCell cellName="Status" column={column} />,
    cell: ({ row }) => (
      <span className="inline-flex items-center gap-2 text-sm">
        <StateIndicator value={row.original.status} isProcessing={isActiveRun(row.original)} kind="buildRun" />
        {row.original.status}
      </span>
    ),
    sortingFn: (rowA, rowB) => rowA.original.status.localeCompare(rowB.original.status),
  },
  {
    accessorKey: 'trigger',
    header: ({ column }) => <SortableCell cellName="Trigger" column={column} />,
    cell: ({ row }) => <span className="text-sm">{row.original.trigger}</span>,
    sortingFn: (rowA, rowB) => rowA.original.trigger.localeCompare(rowB.original.trigger),
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
      const cancellable = isActiveRun(row.original);
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
  isLoading,
  onOpenChange,
}: {
  open: boolean;
  run?: BuildRunView;
  logs: string;
  isLoading: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const description = run
    ? `${run.trigger} run on ${run.branch} is ${run.status}`
    : isLoading
      ? 'Loading build logs...'
      : 'Build logs';

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent
        onOpenAutoFocus={(event) => event.preventDefault()}
        side="top"
        className="mx-auto w-300 max-w-[100vw] rounded-b-md">
        <div className="p-2">
          <SheetHeader>
            <SheetTitle>{run ? `${run.projectNameSnapshot} logs` : 'Build logs'}</SheetTitle>
            <SheetDescription>{description}</SheetDescription>
          </SheetHeader>

          <div className="p-4 pt-0 pb-2">
            <LogViewer logs={isLoading ? '' : logs} autoScroll={false} allowWrap timeStamps />
          </div>
        </div>
      </SheetContent>
    </Sheet>
  );
}

export function isTerminalBuildStatus(status: BuildRunStatus) {
  return (
    status === BuildRunStatus.Succeeded ||
    status === BuildRunStatus.Failed ||
    status === BuildRunStatus.TimedOut ||
    status === BuildRunStatus.Cancelled ||
    status === BuildRunStatus.Interrupted
  );
}
