import { DataTable } from '@/components/ui/data-table';
import { AutoUpdateStatus, DeploymentView, ResourceControlState } from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { useMemo } from 'react';
import { Link, useNavigate } from 'react-router';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useSelectedResources } from '@/lib/atoms';
import { ActionData } from '@/pages/types';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { ContentCard } from '@/components/custom/content-card';
import { HardDrive } from 'lucide-react';
import { formatId } from '@/lib/utils';
import { PlatformStatusCell, UPDATE_STATUS_UI, UpdateStatusIcon } from '@/components/custom/common';
import { HoverCard, HoverCardContent, HoverCardTrigger } from '@/components/ui/hover-card';
import { fromNow } from '@/lib/dayjs.helper';
import { truncate } from '@/lib/truncate';

export const DeploymentsTable = ({
  items,
  actions,
  isLoading,
}: {
  items: DeploymentView[];
  isLoading: boolean;
  actions: Record<
    string,
    React.FC<{ resource: DeploymentView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) => {
  const [_, setSelectedResources] = useSelectedResources<DeploymentView>('Deployment');
  const cols = useMemo(() => columns(actions ?? {}), [actions]);

  return (
    <ContentCard>
      <DataTable columns={cols} data={items ?? []} isLoading={isLoading} onSelectionChange={setSelectedResources} />
    </ContentCard>
  );
};

const columns = (
  actions: Record<
    string,
    React.FC<{ resource: DeploymentView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >,
): ColumnDef<DeploymentView>[] => [
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
        aria-label="Select deployment"
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'name',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => <DeploymentNameRow deployment={row.original} />,
    sortingFn: (rowA: any, rowB: any): number => rowA.original?.name?.localeCompare(rowB.original?.name),
  },
  {
    accessorKey: 'image',
    header: ({ column }) => <SortableCell cellName="Image" column={column} />,
    cell: ({ row }) => {
      if (!row.original.imageName) return null;
      return (
        <div className="flex flex-row items-center gap-2">
          <HardDrive width={13} height={13} className="text-foreground/80" />
          <Link
            to={`/platforms/${row.original.platformId}/images/${formatId(row.original.dockerImageId ?? '')}`}
            title={row.original.imageName}
            className="table-link">
            {truncate(row.original.imageName, 32)}
          </Link>
        </div>
      );
    },
    sortingFn: (rowA, rowB) => (rowA.original.platformName! < rowB.original.platformName! ? 1 : -1),
  },
  {
    accessorKey: 'updateStatus',
    header: ({ column }) => <SortableCell cellName="Update Status" column={column} />,
    cell: ({ row }) => <DeploymentUpdateStatusCell deployment={row.original} />,
    sortingFn: (rowA, rowB) => (rowA.original.autoUpdateState.status! < rowB.original.autoUpdateState.status! ? 1 : -1),
  },
  {
    accessorKey: 'platform',
    header: ({ column }) => <SortableCell cellName="Platform" column={column} />,
    cell: ({ row }) => (
      <PlatformStatusCell
        status={row.original.platformStatus}
        id={row.original.platformId}
        name={row.original.platformName ?? ''}
      />
    ),
    sortingFn: (rowA, rowB) => (rowA.original.platformName! < rowB.original.platformName! ? 1 : -1),
  },
  {
    id: 'actions',
    cell: ({ row }) => <RowActionMenu resource={row.original} actions={actions} />,
  },
];

const DeploymentNameRow = ({ deployment }: { deployment: DeploymentView }) => {
  const navigate = useNavigate();
  function onClick() {
    navigate(`/deployments/edit/${deployment.id}/`);
  }
  return (
    <div className="flex items-center whitespace-nowrap">
      <div className="flex items-center">
        <StateIndicator
          value={deployment.status}
          isProcessing={deployment.controlState === ResourceControlState.Processing}
        />
      </div>
      <span
        className="cursor-pointer hover:underline"
        onClick={onClick}
        title={deployment.name}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            onClick();
          }
        }}
        tabIndex={0}
        role="button"
        aria-label="Show deployment details">
        {truncate(deployment.name ?? '', 32, 'right')}
      </span>
    </div>
  );
};

const DeploymentUpdateStatusCell = ({ deployment }: { deployment: DeploymentView }) => {
  const status = deployment.autoUpdateState.status;
  const { label } = UPDATE_STATUS_UI[status];
  if (deployment.autoUpdateState.status === AutoUpdateStatus.Unknown) {
    return <span className="text-muted-foreground text-sm">{'<none>'}</span>;
  }
  const trigger = (
    <div className="flex items-center gap-2">
      <UpdateStatusIcon updateStatus={status} />
      <span>{label}</span>
    </div>
  );

  const isUpdateAvailable = status === AutoUpdateStatus.UpdateAvailable;

  return (
    <HoverCard openDelay={150} closeDelay={150}>
      <HoverCardTrigger asChild>
        <div className="inline-flex cursor-default items-center">{trigger}</div>
      </HoverCardTrigger>
      <HoverCardContent align="start" className="w-72 p-4 shadow-lg border-border bg-background">
        <div className="flex justify-between items-start mb-4">
          <div className="space-y-1">
            <h4 className="text-sm font-medium leading-none text-foreground">{deployment.imageName}</h4>
            <p className="text-xs text-muted-foreground">Checked {fromNow(deployment.autoUpdateState.lastCheckedAt)}</p>
          </div>
          <UpdateStatusIcon updateStatus={status} />
        </div>

        <div className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-2.5 text-sm">
          <span className="text-muted-foreground text-xs">Current</span>
          <span
            className="font-mono text-xs text-foreground/80 truncate"
            title={deployment.autoUpdateState.currentDigest ?? undefined}>
            {formatId(deployment.autoUpdateState.currentDigest ?? undefined)}
          </span>

          {isUpdateAvailable && (
            <>
              <span className="text-muted-foreground text-xs">Available</span>
              <span
                className="font-mono text-xs text-amber-600 dark:text-amber-500 truncate"
                title={deployment.autoUpdateState.remoteDigest ?? undefined}>
                {formatId(deployment.autoUpdateState.remoteDigest ?? undefined)}
              </span>
            </>
          )}
        </div>
      </HoverCardContent>
    </HoverCard>
  );
};
