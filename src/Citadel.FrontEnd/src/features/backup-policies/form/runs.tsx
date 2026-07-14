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
import { ResourceSelectorField } from '@/components/custom/common';
import { ContentCard } from '@/components/custom/content-card';
import SortableCell from '@/components/custom/sortable-cell';
import { StateIndicator } from '@/components/custom/state-indicator';
import { TimestampCell } from '@/components/custom/timestamp-cell';
import { Button } from '@/components/ui/button';
import { DataTable } from '@/components/ui/data-table';
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { useTaskSheet } from '@/lib/atoms';
import { byteTransform } from '@/lib/bytes.helper';
import type { DateTimeFormatter } from '@/lib/date-time';
import { useMutate, useRead } from '@/lib/hooks';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { ColumnDef } from '@tanstack/react-table';
import { useQueryClient } from '@tanstack/react-query';
import { Ban, FileText, RotateCcw } from 'lucide-react';
import { useCallback, useMemo, useState } from 'react';
import { toast } from 'sonner';

export function BackupPolicyRunsTab({ resource }: { resource: BackupPolicyView }) {
  const [restoreRun, setRestoreRun] = useState<BackupRunView | undefined>();
  const queryClient = useQueryClient();
  const formatDateTime = useProfileDateTimeFormatter();
  const cancelRun = useMutate('cancelBackupRun');
  const { open: openSheet } = useTaskSheet('BackupPolicy');
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

  const handleOpenLogs = useCallback(
    (run: BackupRunView) => {
      openSheet({
        kind: 'backupRunLogs',
        payload: {
          id: run.id,
          name: run.policyNameSnapshot,
          trigger: run.trigger,
          items: run.items,
        },
      });
    },
    [openSheet],
  );

  const columns = useMemo(
    () => runColumns(handleOpenLogs, setRestoreRun, handleCancel, cancelRun.isPending, formatDateTime),
    [cancelRun.isPending, formatDateTime, handleCancel, handleOpenLogs],
  );

  return (
    <div className="flex flex-col gap-4">
      <ContentCard>
        <DataTable columns={columns} data={runs} isLoading={isLoading} />
      </ContentCard>

      {restoreRun && (
        <BackupRestoreDialog
          key={restoreRun.id}
          run={restoreRun}
          onClose={() => setRestoreRun(undefined)}
        />
      )}
    </div>
  );
}

const runColumns = (
  onSelectLog: (run: BackupRunView) => void,
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
    cell: ({ row }) => (
      <span className="text-sm tabular-nums">
        {(row.original.bytesAdded ?? 0) > 0 ? byteTransform(row.original.bytesAdded, 2) : '-'}
      </span>
    ),
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
          <Button type="button" size="icon-sm" variant="ghost" onClick={() => onSelectLog(run)} title="View logs">
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

function BackupRestoreDialog({
  run,
  onClose,
}: {
  run: BackupRunView;
  onClose: () => void;
}) {
  const source = run.sourceSnapshot as BackupSourceSpecDockerVolumeBackupSource;
  const [targetPlatformId, setTargetPlatformId] = useState(source.platformId);
  const [targetVolumeName, setTargetVolumeName] = useState(source.volumeName);
  const [overwriteExisting, setOverwriteExisting] = useState(false);
  const { open: openSheet } = useTaskSheet('BackupPolicy');

  const canSubmit = Boolean(targetPlatformId && targetVolumeName.trim());

  const handleSubmit = () => {
    if (!canSubmit) return;

    openSheet({
      kind: 'backupRestoreRun',
      payload: {
        id: run.id,
        name: targetVolumeName.trim(),
        targetPlatformId,
        targetVolumeName: targetVolumeName.trim(),
        overwriteExisting,
      },
    });
    onClose();
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
          <Button type="button" disabled={!canSubmit} onClick={handleSubmit}>
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
