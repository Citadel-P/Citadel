import {
  BackupRunItemView,
  BackupPolicyView,
  BackupRestoreRunView,
  BackupRestoreStatus,
  BackupRunStatus,
  BackupRunView,
  BackupSnapshotAvailability,
  BackupSourceSpecDockerVolumeBackupSource,
  LookupResourceType,
  PlatformView,
} from '@/api/generated/api.types';
import { ResourceSelectorField } from '@/components/custom/common';
import { AlertMessage } from '@/components/custom/alert-message';
import { ContentCard } from '@/components/custom/content-card';
import { RunStatusBadge } from '@/components/custom/run-status-badge';
import SortableCell from '@/components/custom/sortable-cell';
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
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { useMutate, useRead } from '@/lib/hooks';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { ColumnDef } from '@tanstack/react-table';
import { useQueryClient } from '@tanstack/react-query';
import { HubConnection } from '@microsoft/signalr';
import { Ban, FileText, RotateCcw } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { toast } from 'sonner';

export function BackupPolicyRunsTab({ resource }: { resource: BackupPolicyView }) {
  const [restoreRun, setRestoreRun] = useState<BackupRunView | undefined>();
  const queryClient = useQueryClient();
  const formatDateTime = useProfileDateTimeFormatter();
  const cancelRun = useMutate('cancelBackupRun');
  const cancelRestoreRun = useMutate('cancelBackupRestoreRun');
  const { open: openSheet } = useTaskSheet('BackupPolicy');
  const readArgs = useMemo(() => ({ query: { policyId: resource.id, limit: 50 } }), [resource.id]);
  const { data, isLoading } = useRead('listBackupRuns', readArgs);
  const restoreReadArgs = useMemo(() => ({ query: { policyId: resource.id, limit: 50 } }), [resource.id]);
  const { data: restoreData, isLoading: isRestoreLoading } = useRead('listBackupRestoreRuns', restoreReadArgs);
  const [runs, setRuns] = useState<BackupRunView[] | undefined>();
  const [restoreRuns, setRestoreRuns] = useState<BackupRestoreRunView[] | undefined>();
  const lastFetchedRef = useRef<BackupRunView[]>([]);
  const lastFetchedRestoreRef = useRef<BackupRestoreRunView[]>([]);

  useEffect(() => {
    if (!data) return;

    const newBase = data.data.runs;
    if (newBase !== lastFetchedRef.current) {
      lastFetchedRef.current = newBase;
      setRuns(newBase);
    }
  }, [data]);

  useEffect(() => {
    if (!restoreData) return;

    const newBase = restoreData.data.runs;
    if (newBase !== lastFetchedRestoreRef.current) {
      lastFetchedRestoreRef.current = newBase;
      setRestoreRuns(newBase);
    }
  }, [restoreData]);

  const visibleRestoreRuns = useMemo(() => restoreRuns ?? [], [restoreRuns]);

  const handleBackupRunInfoUpdated = useCallback(
    (run: BackupRunView, action: string) => {
      if (run.backupPolicyId !== resource.id) return;

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
    },
    [resource.id],
  );

  const handleBackupRestoreRunInfoUpdated = useCallback(
    (run: BackupRestoreRunView, action: string) => {
      setRestoreRuns((prev) => {
        const current = prev ?? [];
        if (action === 'delete') {
          return current.filter((item) => item.id !== run.id);
        }

        const index = current.findIndex((item) => item.id === run.id);
        if (index === -1) {
          return sortRestoreRuns([run, ...current]);
        }

        const updated = [...current];
        updated[index] = run;
        return sortRestoreRuns(updated);
      });
    },
    [],
  );

  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.on('BackupRunInfoUpdated', handleBackupRunInfoUpdated);
    },
    [handleBackupRunInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.off('BackupRunInfoUpdated', handleBackupRunInfoUpdated);
    },
    [handleBackupRunInfoUpdated],
  );

  const setupRestoreEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.on('BackupRestoreRunInfoUpdated', handleBackupRestoreRunInfoUpdated);
    },
    [handleBackupRestoreRunInfoUpdated],
  );

  const removeRestoreEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.off('BackupRestoreRunInfoUpdated', handleBackupRestoreRunInfoUpdated);
    },
    [handleBackupRestoreRunInfoUpdated],
  );

  useSignalRGroup({
    groupName: `backup-runs:${resource.id}`,
    setupEventListeners,
    removeEventListeners,
  });

  useSignalRGroup({
    groupName: `backup-restore-runs:${resource.id}`,
    setupEventListeners: setupRestoreEventListeners,
    removeEventListeners: removeRestoreEventListeners,
  });

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

  const handleCancelRestore = useCallback(
    async (run: BackupRestoreRunView) => {
      try {
        await cancelRestoreRun.mutateAsync({ id: run.id } as any);
        await queryClient.invalidateQueries({ queryKey: ['listBackupRestoreRuns'] });
        toast.success('Restore run cancellation requested');
      } catch {
        toast.error(cancelRestoreRun.validationErrors ?? 'Failed to cancel restore run');
      }
    },
    [cancelRestoreRun, queryClient],
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

  const handleOpenRestoreLogs = useCallback(
    (run: BackupRestoreRunView) => {
      openSheet({
        kind: 'backupRestoreRunLogs',
        payload: {
          id: run.id,
          name: run.targetVolumeName,
          status: run.status,
        },
      });
    },
    [openSheet],
  );

  const columns = useMemo(
    () => runColumns(handleOpenLogs, setRestoreRun, handleCancel, cancelRun.isPending, formatDateTime),
    [cancelRun.isPending, formatDateTime, handleCancel, handleOpenLogs],
  );
  const restoreColumns = useMemo(
    () => restoreRunColumns(handleOpenRestoreLogs, handleCancelRestore, cancelRestoreRun.isPending, formatDateTime),
    [cancelRestoreRun.isPending, formatDateTime, handleCancelRestore, handleOpenRestoreLogs],
  );

  return (
    <div className="flex flex-col gap-4">
      <ContentCard>
        <DataTable columns={columns} data={runs ?? []} isLoading={isLoading && !runs} />
      </ContentCard>

      <div className="flex flex-col gap-2">
        <div className="flex items-center justify-between px-1">
          <h3 className="text-sm font-medium">Restore runs</h3>
          <span className="text-xs text-muted-foreground">{visibleRestoreRuns.length} recent</span>
        </div>
        <ContentCard>
          <DataTable columns={restoreColumns} data={visibleRestoreRuns} isLoading={isRestoreLoading && !restoreRuns} />
        </ContentCard>
      </div>

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
    accessorKey: 'items',
    header: ({ column }) => <SortableCell cellName="Volumes" column={column} />,
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

const restoreRunColumns = (
  onSelectLog: (run: BackupRestoreRunView) => void,
  onCancel: (run: BackupRestoreRunView) => void,
  cancelPending: boolean,
  formatDateTime: DateTimeFormatter,
): ColumnDef<BackupRestoreRunView>[] => [
  {
    accessorKey: 'targetVolumeName',
    header: ({ column }) => <SortableCell cellName="Target Volume" column={column} />,
    cell: ({ row }) => <span className="text-sm">{row.original.targetVolumeName}</span>,
    sortingFn: (rowA, rowB) => rowA.original.targetVolumeName.localeCompare(rowB.original.targetVolumeName),
  },
  {
    accessorKey: 'status',
    header: ({ column }) => <SortableCell cellName="Status" column={column} />,
    cell: ({ row }) => <RunStatusBadge status={row.original.status} />,
    sortingFn: (rowA, rowB) => rowA.original.status.localeCompare(rowB.original.status),
  },
  {
    accessorKey: 'overwriteExisting',
    header: ({ column }) => <SortableCell cellName="Mode" column={column} />,
    cell: ({ row }) => (
      <span className="text-sm text-muted-foreground">
        {row.original.overwriteExisting ? 'Overwrite' : 'New volume'}
      </span>
    ),
    sortingFn: (rowA, rowB) => Number(rowA.original.overwriteExisting) - Number(rowB.original.overwriteExisting),
  },
  {
    accessorKey: 'queuedAt',
    header: ({ column }) => <SortableCell cellName="Queued" column={column} />,
    cell: ({ row }) => <TimestampCell value={row.original.queuedAt} formatDateTime={formatDateTime} />,
    sortingFn: (rowA, rowB) => String(rowA.original.queuedAt).localeCompare(String(rowB.original.queuedAt)),
  },
  {
    accessorKey: 'completedAt',
    header: ({ column }) => <SortableCell cellName="Completed" column={column} />,
    cell: ({ row }) =>
      row.original.completedAt ? (
        <TimestampCell value={row.original.completedAt} formatDateTime={formatDateTime} />
      ) : (
        <span className="text-sm text-muted-foreground">-</span>
      ),
    sortingFn: (rowA, rowB) => String(rowA.original.completedAt ?? '').localeCompare(String(rowB.original.completedAt ?? '')),
  },
  {
    id: 'actions',
    cell: ({ row }) => {
      const run = row.original;
      const cancellable = isActiveRestoreRun(run);

      return (
        <div className="flex justify-end gap-1">
          <Button type="button" size="icon-sm" variant="ghost" onClick={() => onSelectLog(run)} title="View logs">
            <FileText className="size-3.5" />
          </Button>
          {cancellable && (
            <Button
              type="button"
              size="icon-sm"
              variant="ghost"
              disabled={cancelPending}
              onClick={() => onCancel(run)}
              title="Cancel restore">
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
  const [overwriteConfirmation, setOverwriteConfirmation] = useState('');
  const { open: openSheet } = useTaskSheet('BackupPolicy');

  const targetName = targetVolumeName.trim();
  const overwriteConfirmed = !overwriteExisting || overwriteConfirmation === targetName;
  const canSubmit = Boolean(targetPlatformId && targetName && overwriteConfirmed);

  const handleSubmit = () => {
    if (!canSubmit) return;

    openSheet({
      kind: 'backupRestoreRun',
      payload: {
        id: run.id,
        name: targetName,
        targetPlatformId,
        targetVolumeName: targetName,
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
              onCheckedChange={(checked) => {
                setOverwriteExisting(checked);
                if (!checked) {
                  setOverwriteConfirmation('');
                }
              }}
            />
            <Label htmlFor="backup-restore-overwrite" className="font-normal">
              Overwrite existing target volume
            </Label>
          </div>
          {overwriteExisting && (
            <div className="flex flex-col gap-3">
              <AlertMessage type="warning" title="Overwrite restore">
                The target volume will be deleted and recreated before the snapshot is restored. This cannot be undone.
              </AlertMessage>
              <div className="flex flex-col gap-2">
                <Label htmlFor="backup-restore-confirmation">Type the target volume name to confirm</Label>
                <Input
                  id="backup-restore-confirmation"
                  value={overwriteConfirmation}
                  onChange={(event) => setOverwriteConfirmation(event.target.value)}
                  placeholder={targetName || 'target volume name'}
                  autoComplete="off"
                />
              </div>
            </div>
          )}
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

  const label = items.length === 1 ? items[0].volumeName : `${items.length} volumes`;

  return <span className="block min-w-0 truncate text-sm">{label}</span>;
}

function isActiveRun(run: Pick<BackupRunView, 'status'>) {
  return (
    run.status === BackupRunStatus.Queued ||
    run.status === BackupRunStatus.Preparing ||
    run.status === BackupRunStatus.Running ||
    run.status === BackupRunStatus.ApplyingRetention
  );
}

function isActiveRestoreRun(run: Pick<BackupRestoreRunView, 'status'>) {
  return (
    run.status === BackupRestoreStatus.Queued ||
    run.status === BackupRestoreStatus.Preparing ||
    run.status === BackupRestoreStatus.Running
  );
}

function canRestore(run: BackupRunView) {
  return run.sourceSnapshot.$type === 'DockerVolume' && run.snapshotAvailability === BackupSnapshotAvailability.Available;
}

function sortRuns(runs: BackupRunView[]) {
  return [...runs].sort((left, right) => String(right.queuedAt).localeCompare(String(left.queuedAt)));
}

function sortRestoreRuns(runs: BackupRestoreRunView[]) {
  return [...runs].sort((left, right) => String(right.queuedAt).localeCompare(String(left.queuedAt)));
}
