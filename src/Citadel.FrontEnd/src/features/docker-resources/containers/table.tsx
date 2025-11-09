import { DataTable } from '@/components/ui/data-table';
import { ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { ContainerView, ContainerStateStatus, ContainerStatView } from '@/api/generated/api.types';
import { truncate } from '@/lib/truncate';
import { toFixedNumber } from '@/lib/utils';
import { byteTransform } from '@/lib/bytes.helper';
import SortableCell from '@/components/custom/sortable-cell';
import { Link } from 'react-router';
import { CopyToClipboard } from '@/components/custom/copy-to-clipboard';
import { PortsDisplay } from '@/components/custom/ports-display';
import { ImageName } from './container-info/ImageName';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useMemo } from 'react';
import { useSelectedResources } from '@/lib/atoms';
import { ActionData } from '@/pages/types';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';

export const ContainersTable = ({
  items,
  isLoading,
  actions,
}: {
  items: ContainerView[];
  isLoading: boolean;
  actions: Record<
    string,
    React.FC<{ resource: ContainerView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) => {
  const rowCount = useMemo(() => items?.length ?? 0, [items]);
  const [_, setSelectedResources] = useSelectedResources<ContainerView>('Container');
  const cols = useMemo(() => columns(actions ?? {}), [actions]);
  return (
    <div className="flex flex-col gap-3">
      <DataTable
        columns={cols}
        data={items ?? []}
        isLoading={isLoading}
        onSelectionChange={setSelectedResources}
      />
      <div className="text-muted-foreground text-sm p-2 font-normal">
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
    React.FC<{ resource: ContainerView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >,
): ColumnDef<ContainerView>[] => [
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
        aria-label="Select row"
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'name',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => (
      <div className="flex items-center gap-0.5">
        {' '}
        {/* row container */}
        <div className="flex items-center">
          <StateIndicator value={row.original.state ?? ContainerStateStatus.Exited} />
        </div>
        <Link to={`../containers/${row.original.containerId?.slice(0, 12)}/logs`} className="table-link">
          {row.original.name ? row.original.name?.slice(1) : ''}
        </Link>
      </div>
    ),
    sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
      return rowA.original.name < rowB.original.name ? 1 : -1;
    },
  },
  
  {
    accessorKey: 'image',
    header: ({ column }) => <SortableCell cellName="Image" column={column} />,
    cell: ({ row }) => <ImageName image={row.original.imageView ?? undefined} />,
  },
  {
    accessorKey: 'containerId',
    header: ({ column }) => <SortableCell cellName="ID" column={column} />,
    cell: ({ row }) => (
      <CopyToClipboard
        textToCopy={row.original.containerId}
        transform={() => row.original.containerId?.slice(0, 12)}
        groupClassName="rowid"
      />
    ),
  },
  {
    accessorKey: 'CPU',
    header: ({ column }) => <SortableCell cellName="Cpu" column={column} />,
    cell: ({ row }) => <CPUCell container={row.original} />,
    sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
      if (!rowA.original.stats || !rowB.original.stats) return 0;
      return rowA.original.stats[0]?.cpuUsage < rowB.original.stats[0]?.cpuUsage ? 1 : -1;
    },
  },
  {
    accessorKey: 'memory',
    header: ({ column }) => <SortableCell cellName="Memory" column={column} />,
    cell: ({ row }) => <MemoryUsageCell container={row.original} />,
    sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
      const cA = rowA.original.lastStats as ContainerStatView;
      const cB = rowB.original.lastStats as ContainerStatView;

      return (cA?.memoryActive ?? 0) < (cB?.memoryActive ?? 0) ? 1 : -1;
    },
  },
  {
    accessorKey: 'ports',
    header: () => <span>Ports</span>,
    cell: ({ row }) => <PortsDisplay ports={row.original.ports} />,
  },

  {
    accessorKey: 'stack',
    header: ({ column }) => <SortableCell cellName="Stack" column={column} />,
    cell: ({ row }) => <span>{row.original.stack && truncate(row.original.stack, 10, 'left')}</span>,
    sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
      return rowA.original.stack < rowB.original.stack ? 1 : -1;
    },
  },
  {
    id: 'actions',
    cell: ({ row }) => <RowActionMenu resource={row.original} actions={actions} />,
  },
];

const MemoryUsageCell = ({ container }: { container: ContainerView }) => {
  if (container.state !== ContainerStateStatus.Running) {
    return <div className="text-muted">0B / 0B</div>;
  }
  return (
    byteTransform(container.lastStats?.memoryActive ?? 0, 2) +
    ' / ' +
    byteTransform(container.lastStats?.memoryLimit ?? 0, 2)
  );
};

const CPUCell = ({ container }: { container: ContainerView }) => {
  if (container.state !== ContainerStateStatus.Running) {
    return <div className="text-muted">0%</div>;
  }
  return container.lastStats?.cpuUsage ? toFixedNumber(container.lastStats?.cpuUsage as number, 'percent') : '0%';
};
