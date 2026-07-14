import { DataTable } from '@/components/ui/data-table';
import { BackupCoverageStatus, DockerVolumeResultView } from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { useMemo, useState } from 'react';
import { byteTransform } from '@/lib/bytes.helper';
import { fromNow } from '@/lib/dayjs.helper';
import { useNavigate, useParams } from 'react-router';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useSelectedResources } from '@/lib/atoms';
import { ActionData } from '@/pages/types';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { ContentCard } from '@/components/custom/content-card';
import { truncate } from '@/lib/truncate';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { useAppContext } from '@/lib/context/app-context';
import { VolumeBrowserSheet } from './volume-browser-sheet';

export const VolumesTable = ({
  items,
  actions,
  isLoading,
}: {
  items: DockerVolumeResultView[];
  isLoading: boolean;
  actions: Record<
    string,
    React.FC<{ resource: DockerVolumeResultView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) => {
  const [_, setSelectedResources] = useSelectedResources<DockerVolumeResultView>('Volume');
  const { currentPlatform } = useAppContext();
  const [browsingVolume, setBrowsingVolume] = useState<DockerVolumeResultView | null>(null);
  const cols = useMemo(() => columns(actions ?? {}, setBrowsingVolume), [actions]);

  return (
    <>
      <ContentCard>
        <DataTable columns={cols} data={items} isLoading={isLoading} onSelectionChange={setSelectedResources} />
      </ContentCard>
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
    React.FC<{ resource: DockerVolumeResultView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >,
  onBrowse: (volume: DockerVolumeResultView) => void,
): ColumnDef<DockerVolumeResultView>[] => [
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
    sortingFn: (rowA: any, rowB: any): number => rowA.original?.name?.localeCompare(rowB.original?.name),
  },
  {
    accessorKey: 'created',
    header: ({ column }) => <SortableCell cellName="Created" column={column} />,
    cell: ({ row }) => <span className="">{fromNow(new Date(row.original.createdAt as any).getTime())}</span>,
    sortingFn: (rowA, rowB) => (rowA.original.createdAt! < rowB.original.createdAt! ? 1 : -1),
  },
  {
    accessorKey: 'backupCoverage.status',
    header: ({ column }) => <SortableCell cellName="Backup" column={column} />,
    cell: ({ row }) => <BackupCoverageBadge volume={row.original} />,
    sortingFn: (rowA, rowB) => backupCoverageRank(rowA.original) - backupCoverageRank(rowB.original),
  },
  {
    accessorKey: 'driver',
    header: ({ column }) => <SortableCell cellName="Driver" column={column} />,
    cell: ({ row }) => <span className="">{row.original.driver}</span>,
    sortingFn: (rowA, rowB) => (rowA.original.createdAt! < rowB.original.createdAt! ? 1 : -1),
  },
  {
    accessorKey: 'scope',
    header: ({ column }) => <SortableCell cellName="Scope" column={column} />,
    cell: ({ row }) => <span className="">{row.original.scope}</span>,
    sortingFn: (rowA, rowB) => (rowA.original.createdAt! < rowB.original.createdAt! ? 1 : -1),
  },
  {
    accessorKey: 'size',
    header: ({ column }) => <SortableCell cellName="Size" column={column} />,
    cell: ({ row }) => <span className="">{byteTransform(row.original.usageData?.size, 2)}</span>,
    sortingFn: (rowA, rowB) => {
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

const backupCoverageRank = (volume: DockerVolumeResultView) => {
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

const BackupCoverageBadge = ({ volume }: { volume: DockerVolumeResultView }) => {
  const coverage = volume.backupCoverage;
  if (!coverage) {
    return <span className="text-muted-foreground text-xs">-</span>;
  }

  const policyCount = Number(coverage.policyCount ?? 0);
  const status = coverage.status;
  const title = [
    `${status}${policyCount > 0 ? `, ${policyCount} policy${policyCount === 1 ? '' : 'ies'}` : ''}`,
    coverage.lastSuccessfulRunAt ? `Last successful: ${coverage.lastSuccessfulRunAt}` : null,
    coverage.lastRunStatus ? `Last run: ${coverage.lastRunStatus}` : null,
  ]
    .filter(Boolean)
    .join('\n');

  return (
    <Badge
      variant="outline"
      title={title}
      className={cn(
        'rounded-sm',
        status === BackupCoverageStatus.Protected &&
          'border-green-500/40 bg-green-500/10 text-green-700 dark:text-green-300',
        status === BackupCoverageStatus.Warning &&
          'border-orange-500/40 bg-orange-500/10 text-orange-700 dark:text-orange-300',
        status === BackupCoverageStatus.Failed && 'border-red-500/40 bg-red-500/10 text-red-700 dark:text-red-300',
        status === BackupCoverageStatus.Unprotected && 'border-muted-foreground/30 bg-muted/50 text-muted-foreground',
      )}>
      {coverageLabel(status)}
      {policyCount > 0 && <span className="text-muted-foreground ml-1">({policyCount})</span>}
    </Badge>
  );
};

const coverageLabel = (status: BackupCoverageStatus) => {
  switch (status) {
    case BackupCoverageStatus.NotApplicable:
      return 'N/A';
    case BackupCoverageStatus.Unprotected:
      return 'Unprotected';
    case BackupCoverageStatus.Protected:
      return 'Protected';
    case BackupCoverageStatus.Warning:
      return 'Warning';
    case BackupCoverageStatus.Failed:
      return 'Failed';
  }
};

const VolumeNameRow = ({ volume }: { volume: DockerVolumeResultView }) => {
  const { platformId } = useParams<{ platformId: string }>();
  const navigate = useNavigate();
  function onClick() {
    navigate(`/platforms/${platformId}/volumes/${volume.id}/`);
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
