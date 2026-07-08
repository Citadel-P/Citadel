import { ActionRunStatus, AutomationActionRunView, AutomationActionView } from '@/api/generated/api.types';
import { ContentCard } from '@/components/custom/content-card';
import { LogViewer } from '@/components/custom/common';
import SortableCell from '@/components/custom/sortable-cell';
import { StateIndicator } from '@/components/custom/state-indicator';
import { Button } from '@/components/ui/button';
import { DataTable } from '@/components/ui/data-table';
import { fromNow } from '@/lib/dayjs.helper';
import { useMutate, useRead } from '@/lib/hooks';
import { ColumnDef } from '@tanstack/react-table';
import { useQueryClient } from '@tanstack/react-query';
import { Ban, FileText } from 'lucide-react';
import { useCallback, useMemo, useState } from 'react';
import { toast } from 'sonner';

export function AutomationActionRunsTab({ resource }: { resource: AutomationActionView }) {
  const [selectedRunId, setSelectedRunId] = useState<string | undefined>();
  const queryClient = useQueryClient();
  const cancelRun = useMutate('cancelAutomationActionRun');
  const { data, isLoading } = useRead(
    'listAutomationActionRuns',
    { id: resource.id, query: { limit: 50 } },
    { refetchInterval: resource.currentRunId ? 3000 : 10000 },
  );

  const runs = useMemo(() => data?.data.runs ?? [], [data?.data.runs]);
  const activeRunId = useMemo(
    () => (runs.some((run) => run.id === selectedRunId) ? selectedRunId : runs[0]?.id),
    [runs, selectedRunId],
  );

  const logs = useRead(
    'getAutomationActionRunLogs',
    { id: resource.id, runId: activeRunId ?? '' },
    { enabled: Boolean(activeRunId) },
  );

  const handleCancel = useCallback(async (run: AutomationActionRunView) => {
    try {
      await cancelRun.mutateAsync({ id: resource.id, runId: run.id } as any);
      await queryClient.invalidateQueries({ queryKey: ['listAutomationActionRuns', { id: resource.id }] });
      await queryClient.invalidateQueries({ queryKey: ['getAutomationAction', { id: resource.id }] });
      toast.success('Run cancellation requested');
    } catch {
      toast.error(cancelRun.validationErrors ?? 'Failed to cancel run');
    }
  }, [cancelRun, queryClient, resource.id]);

  const columns = useMemo(
    () => runColumns(setSelectedRunId, handleCancel, cancelRun.isPending),
    [cancelRun.isPending, handleCancel],
  );
  const selectedRun = runs.find((run) => run.id === activeRunId);

  return (
    <div className="flex flex-col gap-4">
      <ContentCard>
        <DataTable columns={columns} data={runs} isLoading={isLoading} />
      </ContentCard>

      <ContentCard>
        <div className="flex flex-col gap-3 p-4">
          <div className="flex flex-col gap-1">
            <h3 className="text-sm font-medium">Logs</h3>
            <p className="text-xs text-muted-foreground">
              {selectedRun ? `${selectedRun.trigger} run queued ${fromNow(selectedRun.queuedAt)}` : 'No run selected'}
            </p>
          </div>
          <LogViewer logs={logs.data?.data.logs ?? ''} autoScroll={false} allowWrap timeStamps />
        </div>
      </ContentCard>
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
          isProcessing={row.original.status === ActionRunStatus.Running || row.original.status === ActionRunStatus.Queued}
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
      const cancellable =
        row.original.status === ActionRunStatus.Queued || row.original.status === ActionRunStatus.Running;
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
