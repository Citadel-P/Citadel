import { DataTable } from '@/components/ui/data-table';
import { DeploymentView, ResourceControlState } from '@/api/generated/api.types';
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
            to={`/platforms/${row.original.platformId}/images/${formatId(row.original.imageId ?? '')}`}
            className="table-link">
            {row.original.imageName}
          </Link>
        </div>
      );
    },
    sortingFn: (rowA, rowB) => (rowA.original.platformName! < rowB.original.platformName! ? 1 : -1),
  },
  {
    accessorKey: 'updateStatus',
    header: ({ column }) => <SortableCell cellName="Update Status" column={column} />,
    cell: ({ row }) => {
      const status = row.original.autoUpdateState.status;
      const { label } = UPDATE_STATUS_UI[status];

      return (
        <div className="flex items-center gap-2">
          <UpdateStatusIcon updateStatus={status} />
          <span>{label}</span>
        </div>
      );
    },
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
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            onClick();
          }
        }}
        tabIndex={0}
        role="button"
        aria-label="Show deployment details">
        {deployment.name}
      </span>
    </div>
  );
};
