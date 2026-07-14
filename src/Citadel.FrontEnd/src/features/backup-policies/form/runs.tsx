import {
  BackupRunItemStatus,
  BackupRunItemView,
  BackupPolicyView,
  BackupRunStatus,
  BackupRunView,
  BackupSnapshotAvailability,
  BackupSourceSpecDockerVolumeBackupSource,
  LookupResourceType,
  PlatformView,
} from '@/api/generated/api.types';
import { LogViewer, ResourceSelectorField } from '@/components/custom/common';
import { ContentCard } from '@/components/custom/content-card';
import SortableCell from '@/components/custom/sortable-cell';
import { StateIndicator } from '@/components/custom/state-indicator';
import { TimestampCell } from '@/components/custom/timestamp-cell';
import { Button } from '@/components/ui/button';
import { DataTable } from '@/components/ui/data-table';
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from '@/components/ui/sheet';
import { Switch } from '@/components/ui/switch';
import type { DateTimeFormatter } from '@/lib/date-time';
import { useMutate, useRead } from '@/lib/hooks';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { ColumnDef } from '@tanstack/react-table';
import { useQueryClient } from '@tanstack/react-query';
import { Ban, FileText, RotateCcw } from 'lucide-react';
import { useCallback, useMemo, useState } from 'react';
import { toast } from 'sonner';

export function BackupPolicyRunsTab({ resource }: { resource: BackupPolicyView }) {
  const [logRunId, setLogRunId] = useState<string | undefined>();
  const [restoreRun, setRestoreRun] = useState<BackupRunView | undefined>();
  const queryClient = useQueryClient();
  const formatDateTime = useProfileDateTimeFormatter();
  const cancelRun = useMutate('cancelBackupRun');
  const readArgs = useMemo(() => ({ query: { policyId: resource.id, limit: 50 } }), [resource.id]);
  const { data, isLoading } = useRead('listBackupRuns', readArgs, {
    refetchInterval: (query) => {
      const runs = query.state.data?.data.runs ?? [];
      if (runs.some(isActiveRun)) return 3000;
      if (resource.currentRunId && !query.state.data) return 3000;
      return false;
    },
  });

  const runs = useMemo(() => data?.data.runs ?? [], [data?.data.runs]);
  const selectedRun = useMemo(() => runs.find((run) => run.id === logRunId), [logRunId, runs]);
  const logsArgs = useMemo(() => ({ id: logRunId ?? '' }), [logRunId]);
  const logs = useRead('getBackupRunLogs', logsArgs, { enabled: Boolean(logRunId) });

  const handleCancel = useCallback(
    async (run: BackupRunView) => {
      try {
        await cancelRun.mutateAsync({ id: run.id } as any);
        await queryClient.invalidateQueries({ queryKey: ['listBackupRuns'] });
        await queryClient.invalidateQueries({ queryKey: ['getBackupPolicy', { id: resource.id }] });
        toast.success('Backup run cancellation requested');
      } catch {
        toast.error(cancelRun.validationErrors ?? 'Failed to cancel backup run');
      }
    },
    [cancelRun, queryClient, resource.id],
  );

  const columns = useMemo(
    () => runColumns(setLogRunId, setRestoreRun, handleCancel, cancelRun.isPending, formatDateTime),
    [cancelRun.isPending, formatDateTime, handleCancel],
  );

  return (
    <div className="flex flex-col gap-4">
      <ContentCard>
        <DataTable columns={columns} data={runs} isLoading={isLoading} />
      </ContentCard>

      <BackupRunLogsSheet
        open={Boolean(logRunId)}
        run={selectedRun}
        logs={logs.data?.data.logs ?? ''}
        isLoading={logs.isLoading}
        onOpenChange={(open) => {
          if (!open) setLogRunId(undefined);
        }}
      />

      {restoreRun && (
        <BackupRestoreDialog
          key={restoreRun.id}
          run={restoreRun}
          onClose={() => setRestoreRun(undefined)}
          onQueued={async () => {
            await queryClient.invalidateQueries({ queryKey: ['listBackupRestoreRuns'] });
            setRestoreRun(undefined);
          }}
        />
      )}
    </div>
  );
}

const runColumns = (
  onSelectLog: (id: string) => void,
  onRestore: (run: BackupRunView) => void,
  onCancel: (run: BackupRunView) => void,
  cancelPending: boolean,
  formatDateTime: DateTimeFormatter,
): ColumnDef<BackupRunView>[] => [
  {
    accessorKey: 'status',
    header: ({ column }) => <SortableCell cellName="Status" column={column} />,
    cell: ({ row }) => (
      <span className="inline-flex items-center gap-2 text-sm">
        <StateIndicator value={row.original.status} isProcessing={isActiveRun(row.original)} kind="backupRun" />
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
    accessorKey: 'items',
    header: ({ column }) => <SortableCell cellName="Items" column={column} />,
    cell: ({ row }) => <BackupRunItemsSummary items={row.original.items} />,
    sortingFn: (rowA, rowB) => Number(rowA.original.items?.length ?? 0) - Number(rowB.original.items?.length ?? 0),
  },
  {
    accessorKey: 'queuedAt',
    header: ({ column }) => <SortableCell cellName="Queued" column={column} />,
    cell: ({ row }) => <TimestampCell value={row.original.queuedAt} formatDateTime={formatDateTime} />,
    sortingFn: (rowA, rowB) => String(rowA.original.queuedAt).localeCompare(String(rowB.original.queuedAt)),
  },
  {
    accessorKey: 'bytesAdded',
    header: ({ column }) => <SortableCell cellName="Added" column={column} />,
    cell: ({ row }) => <span className="text-sm tabular-nums">{formatBytes(row.original.bytesAdded)}</span>,
    sortingFn: (rowA, rowB) => Number(rowA.original.bytesAdded ?? 0) - Number(rowB.original.bytesAdded ?? 0),
  },
  {
    accessorKey: 'snapshotAvailability',
    header: ({ column }) => <SortableCell cellName="Snapshot" column={column} />,
    cell: ({ row }) => <span className="text-sm">{row.original.snapshotAvailability}</span>,
    sortingFn: (rowA, rowB) => rowA.original.snapshotAvailability.localeCompare(rowB.original.snapshotAvailability),
  },
  {
    id: 'actions',
    cell: ({ row }) => {
      const run = row.original;
      const cancellable = isActiveRun(run);
      const restorable = canRestore(run);

      return (
        <div className="flex justify-end gap-1">
          <Button type="button" size="icon-sm" variant="ghost" onClick={() => onSelectLog(run.id)} title="View logs">
            <FileText className="size-3.5" />
          </Button>
          {restorable && (
            <Button type="button" size="icon-sm" variant="ghost" onClick={() => onRestore(run)} title="Restore volume">
              <RotateCcw className="size-3.5" />
            </Button>
          )}
          {cancellable && (
            <Button
              type="button"
              size="icon-sm"
              variant="ghost"
              disabled={cancelPending}
              onClick={() => onCancel(run)}
              title="Cancel run">
              <Ban className="size-3.5" />
            </Button>
          )}
        </div>
      );
    },
  },
];

function BackupRunLogsSheet({
  open,
  run,
  logs,
  isLoading,
  onOpenChange,
}: {
  open: boolean;
  run?: BackupRunView;
  logs: string;
  isLoading: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const description = run ? `${run.trigger} backup run` : isLoading ? 'Loading backup logs...' : 'Backup run logs';

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent
        onOpenAutoFocus={(event) => event.preventDefault()}
        side="top"
        className="mx-auto w-300 max-w-[100vw] rounded-b-md">
        <div className="p-2">
          <SheetHeader>
            <SheetTitle>{run ? `${run.policyNameSnapshot} logs` : 'Backup logs'}</SheetTitle>
            <SheetDescription>{description}</SheetDescription>
          </SheetHeader>

          <div className="p-4 pt-0 pb-2">
            {run && run.items.length > 0 && <BackupRunItemsList items={run.items} />}
            <LogViewer logs={isLoading ? '' : logs} autoScroll={false} allowWrap timeStamps />
          </div>
        </div>
      </SheetContent>
    </Sheet>
  );
}

function BackupRestoreDialog({
  run,
  onClose,
  onQueued,
}: {
  run: BackupRunView;
  onClose: () => void;
  onQueued: () => Promise<void>;
}) {
  const source = run.sourceSnapshot as BackupSourceSpecDockerVolumeBackupSource;
  const [targetPlatformId, setTargetPlatformId] = useState(source.platformId);
  const [targetVolumeName, setTargetVolumeName] = useState(source.volumeName);
  const [overwriteExisting, setOverwriteExisting] = useState(false);
  const restore = useMutate('restoreBackupVolume');

  const canSubmit = targetPlatformId && targetVolumeName.trim();

  const handleSubmit = async () => {
    if (!canSubmit) return;

    try {
      await restore.mutateAsync({
        id: run.id,
        data: {
          targetPlatformId,
          targetVolumeName: targetVolumeName.trim(),
          overwriteExisting,
        },
      } as any);
      toast.success('Restore run queued');
      await onQueued();
    } catch {
      toast.error(restore.validationErrors ?? 'Failed to queue restore run');
    }
  };

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Restore Volume</DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-4">
          <div className="flex flex-col gap-2">
            <Label>Target Platform</Label>
            <ResourceSelectorField
              targetType={LookupResourceType.Platform}
              selected={targetPlatformId}
              onSelect={(platform: PlatformView | undefined) => setTargetPlatformId(platform?.id ?? '')}
              placeholder="Select Platform"
            />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="backup-restore-volume-name">Target Volume</Label>
            <Input
              id="backup-restore-volume-name"
              value={targetVolumeName}
              onChange={(event) => setTargetVolumeName(event.target.value)}
              placeholder="restored_volume"
            />
          </div>
          <div className="flex items-center gap-2">
            <Switch
              id="backup-restore-overwrite"
              checked={overwriteExisting}
              onCheckedChange={setOverwriteExisting}
            />
            <Label htmlFor="backup-restore-overwrite" className="font-normal">
              Overwrite existing target volume
            </Label>
          </div>
        </div>

        <DialogFooter>
          <Button type="button" variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button type="button" disabled={!canSubmit || restore.isPending} onClick={handleSubmit}>
            Restore
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function BackupRunItemsSummary({ items }: { items: BackupRunItemView[] }) {
  if (!items.length) return <span className="text-muted-foreground text-sm">-</span>;

  const failed = items.some((item) => item.status === BackupRunItemStatus.Failed);
  const running = items.some((item) => item.status === BackupRunItemStatus.Running || item.status === BackupRunItemStatus.Pending);
  const status = failed ? BackupRunItemStatus.Failed : running ? BackupRunItemStatus.Running : BackupRunItemStatus.Succeeded;
  const label = items.length === 1 ? items[0].volumeName : `${items.length} volumes`;

  return (
    <span className="inline-flex min-w-0 items-center gap-2 text-sm">
      <StateIndicator value={status} isProcessing={running} />
      <span className="truncate">{label}</span>
    </span>
  );
}

function BackupRunItemsList({ items }: { items: BackupRunItemView[] }) {
  return (
    <div className="mb-3 grid gap-1 rounded-md border p-2">
      {items.map((item) => (
        <div key={item.id} className="grid grid-cols-[1fr_auto_auto] items-center gap-3 text-xs">
          <span className="truncate font-medium">{item.volumeName}</span>
          <span className="text-muted-foreground tabular-nums">{formatBytes(item.bytesAdded)}</span>
          <span className="inline-flex items-center gap-1.5">
            <StateIndicator value={item.status} isProcessing={item.status === BackupRunItemStatus.Running} />
            {item.status}
          </span>
        </div>
      ))}
    </div>
  );
}

function isActiveRun(run: Pick<BackupRunView, 'status'>) {
  return (
    run.status === BackupRunStatus.Queued ||
    run.status === BackupRunStatus.Preparing ||
    run.status === BackupRunStatus.Running ||
    run.status === BackupRunStatus.ApplyingRetention
  );
}

function canRestore(run: BackupRunView) {
  return run.sourceSnapshot.$type === 'DockerVolume' && run.snapshotAvailability === BackupSnapshotAvailability.Available;
}

function formatBytes(value: BackupRunView['bytesAdded']) {
  const bytes = Number(value ?? 0);
  if (!Number.isFinite(bytes) || bytes <= 0) return '-';

  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let next = bytes;
  let unit = 0;
  while (next >= 1024 && unit < units.length - 1) {
    next /= 1024;
    unit += 1;
  }

  return `${next.toFixed(next >= 10 || unit === 0 ? 0 : 1)} ${units[unit]}`;
}
