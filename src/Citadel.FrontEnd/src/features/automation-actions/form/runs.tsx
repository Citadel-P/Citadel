import {
  ActionRunStatus,
  AutomationActionRunView,
  AuthorizedAction,
} from '@/api/generated/api.types';
import { LogViewer } from '@/components/custom/common';
import SortableCell from '@/components/custom/sortable-cell';
import { StateIndicator } from '@/components/custom/state-indicator';
import { Button } from '@/components/ui/button';
import { DataTable } from '@/components/ui/data-table';
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from '@/components/ui/sheet';
import { fromNow } from '@/lib/dayjs.helper';
import { useMutate, useRead } from '@/lib/hooks';
import { ColumnDef } from '@tanstack/react-table';
import { useQueryClient } from '@tanstack/react-query';
import { Ban, FileText } from 'lucide-react';
import { useCallback, useMemo, useState } from 'react';
import { toast } from 'sonner';

export function AutomationActionRunsTab({ resource }: { resource: AuthorizedAction }) {
  const [logRunId, setLogRunId] = useState<string | undefined>();
  const queryClient = useQueryClient();
  const cancelRun = useMutate('cancelAutomationActionRun');
  const { data, isLoading } = useRead(
    'listAutomationActionRuns',
    { id: resource.id, query: { limit: 50 } },
    {
      refetchInterval: (query) => {
        const runs = query.state.data?.data.runs ?? [];
        if (runs.some(isActiveRun)) return 3000;
        if (resource.currentRunId && !query.state.data) return 3000;
        return false;
      },
    },
  );

  const runs = useMemo(() => data?.data.runs ?? [], [data?.data.runs]);
  const selectedRun = useMemo(() => runs.find((run) => run.id === logRunId), [logRunId, runs]);

  const logs = useRead(
    'getAutomationActionRunLogs',
    { id: resource.id, runId: logRunId ?? '' },
    { enabled: Boolean(logRunId) },
  );

  const handleCancel = useCallback(
    async (run: AutomationActionRunView) => {
      try {
        await cancelRun.mutateAsync({ id: resource.id, runId: run.id } as any);
        await queryClient.invalidateQueries({ queryKey: ['listAutomationActionRuns', { id: resource.id }] });
        await queryClient.invalidateQueries({ queryKey: ['getAutomationAction', { id: resource.id }] });
        toast.success('Run cancellation requested');
      } catch {
        toast.error(cancelRun.validationErrors ?? 'Failed to cancel run');
      }
    },
    [cancelRun, queryClient, resource.id],
  );

  const columns = useMemo(
    () => runColumns(setLogRunId, handleCancel, cancelRun.isPending),
    [cancelRun.isPending, handleCancel],
  );

  return (
    <div className="flex flex-col gap-4">
      <DataTable columns={columns} data={runs} isLoading={isLoading} />

      <AutomationRunLogsSheet
        open={Boolean(logRunId)}
        run={selectedRun}
        logs={logs.data?.data.logs ?? ''}
        isLoading={logs.isLoading}
        onOpenChange={(open) => {
          if (!open) setLogRunId(undefined);
        }}
      />
    </div>
  );
}

const runColumns = (
  onSelectLog: (id: string) => void,
  onCancel: (run: AutomationActionRunView) => void,
  cancelPending: boolean,
): ColumnDef<AutomationActionRunView>[] => [
  {
    accessorKey: 'status',
    header: ({ column }) => <SortableCell cellName="Status" column={column} />,
    cell: ({ row }) => (
      <span className="inline-flex items-center gap-2 text-sm">
        <StateIndicator
          value={row.original.status}
          isProcessing={isActiveRun(row.original)}
          kind="automationActionRun"
        />
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
    accessorKey: 'queuedAt',
    header: ({ column }) => <SortableCell cellName="Queued" column={column} />,
    cell: ({ row }) => <span className="text-sm">{fromNow(row.original.queuedAt)}</span>,
    sortingFn: (rowA, rowB) => String(rowA.original.queuedAt).localeCompare(String(rowB.original.queuedAt)),
  },
  {
    accessorKey: 'durationMs',
    header: ({ column }) => <SortableCell cellName="Duration" column={column} />,
    cell: ({ row }) => <span className="text-sm">{formatDuration(row.original.durationMs)}</span>,
    sortingFn: (rowA, rowB) => Number(rowA.original.durationMs ?? 0) - Number(rowB.original.durationMs ?? 0),
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
          <Button
            type="button"
            size="icon-sm"
            variant="ghost"
            onClick={() => onSelectLog(row.original.id)}
            title="View logs">
            <FileText className="size-3.5" />
          </Button>
          {cancellable && (
            <Button
              type="button"
              size="icon-sm"
              variant="ghost"
              disabled={cancelPending}
              onClick={() => onCancel(row.original)}
              title="Cancel run">
              <Ban className="size-3.5" />
            </Button>
          )}
        </div>
      );
    },
  },
];

function formatDuration(value: AutomationActionRunView['durationMs']) {
  if (value === null || value === undefined) return '-';

  const ms = Number(value);
  if (!Number.isFinite(ms)) return '-';
  if (ms < 1000) return `${ms} ms`;

  return `${(ms / 1000).toFixed(1)} s`;
}

function isActiveRun(run: Pick<AutomationActionRunView, 'status'>) {
  return run.status === ActionRunStatus.Queued || run.status === ActionRunStatus.Running;
}

function AutomationRunLogsSheet({
  open,
  run,
  logs,
  isLoading,
  onOpenChange,
}: {
  open: boolean;
  run?: AutomationActionRunView;
  logs: string;
  isLoading: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const description = run
    ? `${run.trigger} run queued ${fromNow(run.queuedAt)}`
    : isLoading
      ? 'Loading run logs...'
      : 'Run logs';

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent
        onOpenAutoFocus={(event) => event.preventDefault()}
        side="top"
        className="mx-auto w-300 max-w-[100vw] rounded-b-md">
        <div className="p-2">
          <SheetHeader>
            <SheetTitle>{run ? `${run.actionName} logs` : 'Run logs'}</SheetTitle>
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
