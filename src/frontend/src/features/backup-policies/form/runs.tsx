import { useSearchParams } from 'react-router';
import {
  BackupSourceSpec,
  BackupRunItemView,
  BackupPolicyView,
  BackupRestoreRunView,
  BackupRestoreStatus,
  BackupRunItemStatus,
  BackupRunStatus,
  BackupRunView,
  BackupSnapshotAvailability,
  LookupResourceType,
  PlatformType,
  PlatformView,
} from '@/api/generated/api.types';
import { ResourceSelectorField } from '@/components/custom/common';
import { AlertMessage } from '@/components/custom/alert-message';
import { RunStatusBadge } from '@/components/custom/run-status-badge';
import SortableCell from '@/components/custom/sortable-cell';
import { Timestamp } from '@/components/custom/timestamp';
import { Button } from '@/components/ui/button';
import { DataTable } from '@/components/ui/data-table';
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { useTaskSheet } from '@/lib/atoms';
import { byteTransform } from '@/lib/bytes.helper';
import type { DateTimeFormatter } from '@/lib/date-time';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useMutate, useRead } from '@/lib/hooks';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { StockFeatures, ColumnDef } from '@tanstack/react-table';
import { useQueryClient } from '@tanstack/react-query';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { Ban, FileText, RotateCcw } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { toast } from 'sonner';

export function BackupPolicyRunsTab({ resource }: { resource: BackupPolicyView }) {
  const [search] = useSearchParams();
  const linkedRunId = search.get('runId');
  const linkedRun = useRead('getBackupRun', { id: linkedRunId ?? undefined });
  const openedLinkedRun = useRef<string | undefined>(undefined);
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

  const handleBackupRestoreRunInfoUpdated = useCallback((run: BackupRestoreRunView, action: string) => {
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
  }, []);

  const setupEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.on('BackupRunInfoUpdated', handleBackupRunInfoUpdated);
    },
    [handleBackupRunInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.off('BackupRunInfoUpdated', handleBackupRunInfoUpdated);
    },
    [handleBackupRunInfoUpdated],
  );

  const setupRestoreEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.on('BackupRestoreRunInfoUpdated', handleBackupRestoreRunInfoUpdated);
    },
    [handleBackupRestoreRunInfoUpdated],
  );

  const removeRestoreEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.off('BackupRestoreRunInfoUpdated', handleBackupRestoreRunInfoUpdated);
    },
    [handleBackupRestoreRunInfoUpdated],
  );

  useRealtimeGroup({
    groupName: `backup-runs:${resource.id}`,
    setupEventListeners,
    removeEventListeners,
  });

  useRealtimeGroup({
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

  useEffect(() => {
    const run = linkedRun.data?.data;
    if (run && run.backupPolicyId === resource.id && run.id === linkedRunId && openedLinkedRun.current !== run.id) {
      openedLinkedRun.current = run.id;
      handleOpenLogs(run);
    }
  }, [linkedRun.data, linkedRunId, resource.id, handleOpenLogs]);

  const handleOpenRestoreLogs = useCallback(
    (run: BackupRestoreRunView) => {
      openSheet({
        kind: 'backupRestoreRunLogs',
        payload: {
          id: run.id,
          name: run.targetHostPath ?? run.targetVolumeName,
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
      <DataTable columns={columns} data={runs ?? []} isLoading={isLoading && !runs} />

      <div className="flex flex-col gap-2">
        <div className="flex items-center justify-between px-1">
          <h3 className="text-sm font-medium">Restore runs</h3>
          <span className="text-xs text-muted-foreground">{visibleRestoreRuns.length} recent</span>
        </div>
        <DataTable columns={restoreColumns} data={visibleRestoreRuns} isLoading={isRestoreLoading && !restoreRuns} />
      </div>

      {restoreRun && (
        <BackupRestoreDialog key={restoreRun.id} run={restoreRun} onClose={() => setRestoreRun(undefined)} />
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
): ColumnDef<StockFeatures, BackupRunView>[] => [
  {
    accessorKey: 'trigger',
    header: ({ column }) => <SortableCell cellName="Trigger" column={column} />,
    cell: ({ row }) => <span className="text-sm">{row.original.trigger}</span>,
    sortFn: (rowA, rowB) => rowA.original.trigger.localeCompare(rowB.original.trigger),
  },
  {
    accessorKey: 'status',
    header: ({ column }) => <SortableCell cellName="Status" column={column} />,
    cell: ({ row }) => <RunStatusBadge status={row.original.status} />,
    sortFn: (rowA, rowB) => rowA.original.status.localeCompare(rowB.original.status),
  },
  {
    accessorKey: 'items',
    header: ({ column }) => <SortableCell cellName="Sources" column={column} />,
    cell: ({ row }) => <BackupRunItemsSummary items={row.original.items} />,
    sortFn: (rowA, rowB) => Number(rowA.original.items?.length ?? 0) - Number(rowB.original.items?.length ?? 0),
  },
  {
    accessorKey: 'queuedAt',
    header: ({ column }) => <SortableCell cellName="Queued" column={column} />,
    cell: ({ row }) => <Timestamp value={row.original.queuedAt} formatDateTime={formatDateTime} />,
    sortFn: (rowA, rowB) => String(rowA.original.queuedAt).localeCompare(String(rowB.original.queuedAt)),
  },
  {
    accessorKey: 'bytesAdded',
    header: ({ column }) => <SortableCell cellName="Added" column={column} />,
    cell: ({ row }) => (
      <span className="text-sm tabular-nums">
        {Number(row.original.bytesAdded ?? 0) > 0 ? byteTransform(row.original.bytesAdded, 2) : '-'}
      </span>
    ),
    sortFn: (rowA, rowB) => Number(rowA.original.bytesAdded ?? 0) - Number(rowB.original.bytesAdded ?? 0),
  },
  {
    accessorKey: 'snapshotAvailability',
    header: ({ column }) => <SortableCell cellName="Snapshot" column={column} />,
    cell: ({ row }) => <span className="text-sm">{row.original.snapshotAvailability}</span>,
    sortFn: (rowA, rowB) => rowA.original.snapshotAvailability.localeCompare(rowB.original.snapshotAvailability),
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
            <Button type="button" size="icon-sm" variant="ghost" onClick={() => onRestore(run)} title="Restore backup">
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
): ColumnDef<StockFeatures, BackupRestoreRunView>[] => [
  {
    accessorKey: 'targetVolumeName',
    header: ({ column }) => <SortableCell cellName="Target" column={column} />,
    cell: ({ row }) => <span className="text-sm">{row.original.targetHostPath ?? row.original.targetVolumeName}</span>,
    sortFn: (rowA, rowB) =>
      (rowA.original.targetHostPath ?? rowA.original.targetVolumeName).localeCompare(
        rowB.original.targetHostPath ?? rowB.original.targetVolumeName,
      ),
  },
  {
    accessorKey: 'status',
    header: ({ column }) => <SortableCell cellName="Status" column={column} />,
    cell: ({ row }) => <RunStatusBadge status={row.original.status} />,
    sortFn: (rowA, rowB) => rowA.original.status.localeCompare(rowB.original.status),
  },
  {
    accessorKey: 'overwriteExisting',
    header: ({ column }) => <SortableCell cellName="Mode" column={column} />,
    cell: ({ row }) => (
      <span className="text-sm text-muted-foreground">
        {row.original.overwriteExisting
          ? 'Overwrite'
          : row.original.targetHostPath
            ? 'Empty host directory'
            : 'New volume'}
      </span>
    ),
    sortFn: (rowA, rowB) => Number(rowA.original.overwriteExisting) - Number(rowB.original.overwriteExisting),
  },
  {
    accessorKey: 'queuedAt',
    header: ({ column }) => <SortableCell cellName="Queued" column={column} />,
    cell: ({ row }) => <Timestamp value={row.original.queuedAt} formatDateTime={formatDateTime} />,
    sortFn: (rowA, rowB) => String(rowA.original.queuedAt).localeCompare(String(rowB.original.queuedAt)),
  },
  {
    accessorKey: 'completedAt',
    header: ({ column }) => <SortableCell cellName="Completed" column={column} />,
    cell: ({ row }) =>
      row.original.completedAt ? (
        <Timestamp value={row.original.completedAt} formatDateTime={formatDateTime} />
      ) : (
        <span className="text-sm text-muted-foreground">-</span>
      ),
    sortFn: (rowA, rowB) =>
      String(rowA.original.completedAt ?? '').localeCompare(String(rowB.original.completedAt ?? '')),
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

export function BackupRestoreDialog({ run, onClose }: { run: BackupRunView; onClose: () => void }) {
  const source = run.sourceSnapshot;
  const directBindSource = source.$type === 'BindMount' ? source : undefined;
  const successfulItems = useMemo(
    () => run.items.filter((item) => item.status === BackupRunItemStatus.Succeeded && Boolean(item.resticSnapshotId)),
    [run.items],
  );
  const directSource =
    source.$type === 'DockerVolume' ? (source as Extract<BackupSourceSpec, { $type: 'DockerVolume' }>) : undefined;
  const initialSourceItem = !directSource && successfulItems.length === 1 ? successfulItems[0] : undefined;
  const [sourceBackupRunItemId, setSourceBackupRunItemId] = useState(initialSourceItem?.id ?? '');
  const [targetPlatformId, setTargetPlatformId] = useState(
    initialSourceItem?.platformId ?? directSource?.platformId ?? directBindSource?.platformId ?? '',
  );
  const [targetDockerNodeId, setTargetDockerNodeId] = useState(
    initialSourceItem?.dockerNodeId ?? directSource?.dockerNodeId ?? directBindSource?.dockerNodeId ?? '',
  );
  const [targetVolumeName, setTargetVolumeName] = useState(
    initialSourceItem?.hostPath ??
      directBindSource?.hostPath ??
      initialSourceItem?.volumeName ??
      directSource?.volumeName ??
      '',
  );
  const selectedSourceItem = successfulItems.find((item) => item.id === sourceBackupRunItemId);
  const bindSource = Boolean(selectedSourceItem?.hostPath ?? directBindSource?.hostPath);
  const hasBindSnapshots = successfulItems.some((item) => item.hostPath);
  const [overwriteExisting, setOverwriteExisting] = useState(false);
  const [overwriteConfirmation, setOverwriteConfirmation] = useState('');
  const { open: openSheet } = useTaskSheet('BackupPolicy');
  const targetPlatformArgs = useMemo(() => ({ id: targetPlatformId }), [targetPlatformId]);
  const targetPlatform = useRead('getPlatfom', targetPlatformArgs, { enabled: Boolean(targetPlatformId) });
  const isTargetSwarm = targetPlatform.data?.data.type === PlatformType.DockerSwarm;
  const targetNodesArgs = useMemo(() => ({ platformId: targetPlatformId }), [targetPlatformId]);
  const targetNodes = useRead('listSwarmNodes', targetNodesArgs, {
    enabled: Boolean(targetPlatformId) && isTargetSwarm,
  });
  const nodes = targetNodes.data?.data.items ?? [];
  const selectedNode = nodes.find((node) => node.id === targetDockerNodeId);

  const targetName = targetVolumeName.trim();
  const overwriteConfirmed = !overwriteExisting || overwriteConfirmation === targetName;
  const sourceItemSelected = Boolean(directSource) || successfulItems.length <= 1 || Boolean(sourceBackupRunItemId);
  const targetNodeSelected = !isTargetSwarm || Boolean(targetDockerNodeId);
  const canSubmit = Boolean(
    targetPlatformId &&
    targetPlatform.data?.data &&
    targetName &&
    sourceItemSelected &&
    targetNodeSelected &&
    overwriteConfirmed,
  );

  const handleSubmit = () => {
    if (!canSubmit) return;

    openSheet({
      kind: 'backupRestoreRun',
      payload: {
        id: run.id,
        name: targetName,
        targetPlatformId,
        targetVolumeName: bindSource ? '' : targetName,
        targetHostPath: bindSource ? targetName : null,
        overwriteExisting,
        targetDockerNodeId: isTargetSwarm ? targetDockerNodeId : null,
        sourceBackupRunItemId: sourceBackupRunItemId || null,
      },
    });
    onClose();
  };

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{bindSource ? 'Restore bind mount' : 'Restore Volume'}</DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-4">
          {!directSource && successfulItems.length > 1 && (
            <div className="flex flex-col gap-2">
              <Label htmlFor="backup-restore-source-item">
                {hasBindSnapshots ? 'Snapshot source' : 'Snapshot Volume'}
              </Label>
              <ResourceSelectorField
                id="backup-restore-source-item"
                items={successfulItems.map((item) => ({ ...item, name: formatBackupRunItem(item) }))}
                selected={sourceBackupRunItemId}
                allowClear={false}
                placeholder="Select a successful snapshot"
                searchPlaceholder="Search snapshots..."
                onSelect={(item) => {
                  if (!item) return;
                  setSourceBackupRunItemId(item.id);
                  setTargetPlatformId(item.platformId);
                  setTargetDockerNodeId(item.dockerNodeId ?? '');
                  setTargetVolumeName(item.hostPath ?? item.volumeName);
                  setOverwriteExisting(false);
                  setOverwriteConfirmation('');
                }}
              />
            </div>
          )}
          <div className="flex flex-col gap-2">
            <Label>Target Platform</Label>
            <ResourceSelectorField
              aria-label="Target Platform"
              targetType={LookupResourceType.Platform}
              selected={targetPlatformId}
              onSelect={(platform: PlatformView | undefined) => {
                setTargetPlatformId(platform?.id ?? '');
                setTargetDockerNodeId('');
                setOverwriteExisting(false);
                setOverwriteConfirmation('');
              }}
              placeholder="Select Platform"
            />
          </div>
          {isTargetSwarm && (
            <div className="flex flex-col gap-2">
              <Label htmlFor="backup-restore-target-node">Target Node</Label>
              <ResourceSelectorField
                id="backup-restore-target-node"
                items={nodes
                  .filter((node) => !node.isStale && node.status.toLowerCase() === 'ready')
                  .map((node) => ({ id: node.id, name: node.hostname || node.id }))}
                selected={
                  targetDockerNodeId
                    ? { id: targetDockerNodeId, name: selectedNode?.hostname || targetDockerNodeId }
                    : undefined
                }
                allowClear={false}
                disabled={targetNodes.isLoading}
                placeholder={targetNodes.isLoading ? 'Loading Nodes...' : 'Select a Node'}
                searchPlaceholder="Search Nodes..."
                onSelect={(node) => setTargetDockerNodeId(node?.id ?? '')}
              />
              <p className="text-xs text-muted-foreground">
                {bindSource
                  ? 'The host path must exist on this Node. Citadel does not reattach Services.'
                  : 'The restored local Volume is created only on this Swarm Node. Citadel does not reattach Services.'}
              </p>
            </div>
          )}
          <div className="flex flex-col gap-2">
            <Label htmlFor="backup-restore-volume-name">{bindSource ? 'Target host path' : 'Target Volume'}</Label>
            <Input
              id="backup-restore-volume-name"
              value={targetVolumeName}
              onChange={(event) => setTargetVolumeName(event.target.value)}
              placeholder={bindSource ? '/srv/app/recovered-data' : 'restored_volume'}
            />
          </div>
          {bindSource && (
            <p className="text-xs text-muted-foreground">
              Prepare an existing empty directory, or an existing file with overwrite enabled, on the target host. Its
              type must match the snapshot. Stop writers before restoring. Directory restores merge files; unrelated
              files remain.
            </p>
          )}
          {(!isTargetSwarm || bindSource) && (
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
                {bindSource ? 'Allow overwriting existing host files' : 'Overwrite existing target volume'}
              </Label>
            </div>
          )}
          {overwriteExisting && (
            <div className="flex flex-col gap-3">
              <AlertMessage type="warning" title="Overwrite restore">
                {bindSource
                  ? 'Files at the target host path may be replaced. This cannot be undone.'
                  : 'Existing files in the target volume will be replaced by the snapshot. This cannot be undone.'}
              </AlertMessage>
              <div className="flex flex-col gap-2">
                <Label htmlFor="backup-restore-confirmation">
                  {bindSource ? 'Type the target host path to confirm' : 'Type the target volume name to confirm'}
                </Label>
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

  const label = items.length === 1 ? formatBackupRunItem(items[0]) : `${items.length} sources`;

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
  if (run.snapshotAvailability !== BackupSnapshotAvailability.Available) return false;
  if (run.sourceSnapshot.$type === 'DockerVolume') return true;

  return run.items.some((item) => item.status === BackupRunItemStatus.Succeeded && Boolean(item.resticSnapshotId));
}

function formatBackupRunItem(item: BackupRunItemView) {
  const node = item.nodeHostname ?? item.dockerNodeId;
  const name = item.hostPath ?? item.volumeName;
  return node ? `${name} on ${node}` : name;
}

function sortRuns(runs: BackupRunView[]) {
  return [...runs].sort((left, right) => String(right.queuedAt).localeCompare(String(left.queuedAt)));
}

function sortRestoreRuns(runs: BackupRestoreRunView[]) {
  return [...runs].sort((left, right) => String(right.queuedAt).localeCompare(String(left.queuedAt)));
}
