import { DataTable } from '@/components/ui/data-table';
import { BackupCoverageStatus, VolumeView, PlatformType } from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { StockFeatures, ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { useMemo, useState } from 'react';
import { byteTransform } from '@/lib/bytes.helper';
import { Timestamp } from '@/components/custom/timestamp';
import type { DateTimeFormatter } from '@/lib/date-time';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { useLocation, useNavigate, useParams } from 'react-router';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useSelectedResources } from '@/lib/atoms';
import { ActionData } from '@/pages/types';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { truncate } from '@/lib/truncate';
import { ResourceBackupIndicator } from '@/features/backup-policies/resource-backup-indicator';
import { useAppContext } from '@/lib/context/app-context';
import { VolumeBrowserSheet } from './volume-browser-sheet';
import { createNodeResourceColumn } from '@/components/custom/node-resource-column';

export const VolumesTable = ({
  items,
  actions,
  isLoading,
}: {
  items: VolumeView[];
  isLoading: boolean;
  actions: Record<
    string,
    React.FC<{ resource: VolumeView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) => {
  const [_, setSelectedResources] = useSelectedResources<VolumeView>('Volume');
  const formatDateTime = useProfileDateTimeFormatter();
  const { currentPlatform } = useAppContext();
  const [browsingVolume, setBrowsingVolume] = useState<VolumeView | null>(null);
  const showNode = currentPlatform?.type === PlatformType.DockerSwarm;
  const cols = useMemo(
    () => columns(actions ?? {}, setBrowsingVolume, showNode, formatDateTime),
    [actions, showNode, formatDateTime],
  );

  return (
    <>
      <DataTable columns={cols} data={items} isLoading={isLoading} onSelectionChange={setSelectedResources} />
      <VolumeBrowserSheet
        open={!!browsingVolume}
        onOpenChange={(open) => !open && setBrowsingVolume(null)}
        volume={browsingVolume}
        platformId={currentPlatform?.id}
      />
    </>
  );
};

const columns = (
  actions: Record<
    string,
    React.FC<{ resource: VolumeView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >,
  onBrowse: (volume: VolumeView) => void,
  showNode: boolean,
  formatDateTime: DateTimeFormatter,
): ColumnDef<StockFeatures, VolumeView>[] => [
  {
    id: 'select',
    header: ({ table }) => (
      <Checkbox
        checked={table.getIsAllPageRowsSelected() || (table.getIsSomePageRowsSelected() && 'indeterminate')}
        onCheckedChange={(value) => table.toggleAllPageRowsSelected(!!value)}
        aria-label="Select all"
      />
    ),
    cell: ({ row }) => (
      <Checkbox
        checked={row.getIsSelected()}
        onCheckedChange={(value) => row.toggleSelected(!!value)}
        aria-label="Select volume"
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'name',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => <VolumeNameRow volume={row.original} />,
    sortFn: (rowA: any, rowB: any): number => rowA.original?.name?.localeCompare(rowB.original?.name),
  },
  ...(showNode ? [createNodeResourceColumn<VolumeView>()] : []),
  {
    accessorKey: 'created',
    header: ({ column }) => <SortableCell cellName="Created" column={column} />,
    cell: ({ row }) => <Timestamp value={row.original.createdAt} formatDateTime={formatDateTime} />,
    sortFn: (rowA, rowB) => (rowA.original.createdAt! < rowB.original.createdAt! ? 1 : -1),
  },
  {
    accessorKey: 'backupCoverage.status',
    header: ({ column }) => <SortableCell cellName="Backup" column={column} />,
    cell: ({ row }) => <BackupCoverageBadge volume={row.original} />,
    sortFn: (rowA, rowB) => backupCoverageRank(rowA.original) - backupCoverageRank(rowB.original),
  },
  {
    accessorKey: 'driver',
    header: ({ column }) => <SortableCell cellName="Driver" column={column} />,
    cell: ({ row }) => <span className="">{row.original.driver}</span>,
    sortFn: (rowA, rowB) => (rowA.original.createdAt! < rowB.original.createdAt! ? 1 : -1),
  },
  {
    accessorKey: 'scope',
    header: ({ column }) => <SortableCell cellName="Scope" column={column} />,
    cell: ({ row }) => <span className="">{row.original.scope}</span>,
    sortFn: (rowA, rowB) => (rowA.original.createdAt! < rowB.original.createdAt! ? 1 : -1),
  },
  {
    accessorKey: 'size',
    header: ({ column }) => <SortableCell cellName="Size" column={column} />,
    cell: ({ row }) => <span className="">{byteTransform(row.original.usageData?.size, 2)}</span>,
    sortFn: (rowA, rowB) => {
      const sizeA = rowA.original.usageData?.size ?? 0;
      const sizeB = rowB.original.usageData?.size ?? 0;
      return sizeA < sizeB ? 1 : -1;
    },
  },
  {
    id: 'actions',
    cell: ({ row }) => (
      <RowActionMenu
        resource={row.original}
        actions={actions}
        onAction={(action) => {
          if (action.key === 'browse') onBrowse(row.original);
        }}
      />
    ),
  },
];

const backupCoverageRank = (volume: VolumeView) => {
  switch (volume.backupCoverage?.status) {
    case BackupCoverageStatus.Failed:
      return 0;
    case BackupCoverageStatus.Warning:
      return 1;
    case BackupCoverageStatus.Unprotected:
      return 2;
    case BackupCoverageStatus.Protected:
      return 3;
    case BackupCoverageStatus.NotApplicable:
      return 4;
    default:
      return 5;
  }
};

const BackupCoverageBadge = ({ volume }: { volume: VolumeView }) => {
  const { platformId } = useParams<{ platformId: string }>();
  const { currentPlatform } = useAppContext();
  const location = useLocation();
  const platform = platformId ?? currentPlatform?.id;
  const coverage = volume.backupCoverage;
  if (!platform) return null;
  return (
    <ResourceBackupIndicator
      compact
      source={{
        $type: 'DockerVolume',
        platformId: platform,
        volumeName: volume.name,
        dockerNodeId: volume.dockerNodeId,
      }}
      returnTo={`${location.pathname}${location.search}${location.hash}`}
      coverage={
        coverage
          ? {
              status: coverage.status,
              platformId: platform,
              volumes: [
                {
                  name: volume.name,
                  dockerNodeId: volume.dockerNodeId,
                  nodeHostname: volume.nodeHostname,
                  includedInResourcePolicy: false,
                  status: coverage.status,
                  policies: coverage.policies ?? [],
                  lastSuccessfulRunAt: coverage.lastSuccessfulRunAt,
                },
              ],
              policies: coverage.policies ?? [],
              canConfigure: coverage.canConfigure ?? false,
              hasResourcePolicy: false,
              unavailable: false,
              warnings: [],
              reasons: coverage.reasons ?? [],
            }
          : undefined
      }
    />
  );
};

const VolumeNameRow = ({ volume }: { volume: VolumeView }) => {
  const { platformId } = useParams<{ platformId: string }>();
  const navigate = useNavigate();
  function onClick() {
    const nodeQuery = volume.dockerNodeId ? `?dockerNodeId=${encodeURIComponent(volume.dockerNodeId)}` : '';
    navigate(`/platforms/${platformId}/volumes/${volume.id}/${nodeQuery}`);
  }
  return (
    <div className="flex items-center whitespace-nowrap">
      <div className="flex items-center">
        <StateIndicator value={volume.inUse} />
      </div>
      <span
        className="cursor-pointer hover:underline"
        onClick={onClick}
        title={volume.id}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            onClick();
          }
        }}
        tabIndex={0}
        role="button"
        aria-label="Show volume details">
        {truncate(volume.id ?? '', 32, 'right')}
      </span>
    </div>
  );
};
