import { DataTable } from '@/components/ui/data-table';
import { DeploymentView } from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { useMemo } from 'react';
import { fromNow } from '@/lib/dayjs.helper';
import { useNavigate } from 'react-router';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useSelectedResources } from '@/lib/atoms';
import { ActionData } from '@/pages/types';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';

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

  return <DataTable columns={cols} data={items ?? []} isLoading={isLoading} onSelectionChange={setSelectedResources} />;
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
    accessorKey: 'created',
    header: ({ column }) => <SortableCell cellName="Created" column={column} />,
    cell: ({ row }) => <span className="">{fromNow(new Date(row.original.createdAt as any).getTime())}</span>,
    sortingFn: (rowA, rowB) => (rowA.original.createdAt! < rowB.original.createdAt! ? 1 : -1),
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
        <StateIndicator value={deployment.activeVersion?.status} />
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
        {deployment.id}
      </span>
    </div>
  );
};
