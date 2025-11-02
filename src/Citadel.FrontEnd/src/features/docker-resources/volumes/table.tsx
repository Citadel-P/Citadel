import { DataTable } from '@/components/ui/data-table';
import { DockerVolumeResult } from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { useMemo } from 'react';
import { byteTransform } from '@/lib/bytes.helper';
import { fromNow } from '@/lib/dayjs.helper';
import { useNavigate, useParams } from 'react-router';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useSelectedResources } from '@/lib/atoms';
import { ActionData } from '@/pages/types';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';

export const VolumesTable = ({
  items,
  actions,
  isLoading,
}: {
  items: DockerVolumeResult[];
  isLoading: boolean;
  actions: Record<
    string,
    React.FC<{ resource: DockerVolumeResult; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) => {
  const rowCount = useMemo(() => items?.length ?? 0, [items]);
  const [_, setSelectedResources] = useSelectedResources<DockerVolumeResult>('Volume');

  return (
    <div className="flex flex-col gap-3">
      <DataTable
        columns={columns(actions ?? {})}
        data={items ?? []}
        isLoading={isLoading}
        onSelectionChange={setSelectedResources}
      />
      <div className="text-muted-foreground text-xs p-2 font-normal">
        {rowCount > 0 && (
          <span>
            Showing {rowCount} of {rowCount} volume(s)
          </span>
        )}
      </div>
    </div>
  );
};

const columns = (
  actions: Record<
    string,
    React.FC<{ resource: DockerVolumeResult; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >,
): ColumnDef<DockerVolumeResult>[] => [
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
    cell: ({ row }) => <RowActionMenu resource={row.original} actions={actions} />,
  },
];

const VolumeNameRow = ({ volume }: { volume: DockerVolumeResult }) => {
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
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            onClick();
          }
        }}
        tabIndex={0}
        role="button"
        aria-label="Show volume details">
        {volume.id}
      </span>
    </div>
  );
};
