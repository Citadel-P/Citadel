import { DataTable } from '@/components/ui/data-table';
import { ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { useContainersContext } from './ContainersContext';
import { ContainerView, ContainerStateStatus, ContainerStatView } from '@/api/generated/api.types';
import { truncate } from '@/lib/truncate';
import { toFixedNumber } from '@/lib/utils';
import { byteTransform } from '@/lib/bytes.helper';
import SortableCell from '@/components/ui/SortableCell';
import { Link } from 'react-router';
import { TableDropdown } from './table-dropdown';
import { CopyTextToClipboard } from '@/components/ui/CopyTextToClipboard';
import { PortsDisplay } from '@/components/ui/PortsDisplay';
import { ImageName } from './container-info/ImageName';
import { StateIndicator } from '@/components/custom/state-indicator';

const columns: ColumnDef<ContainerView>[] = [
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
    accessorKey: 'containerId',
    header: ({ column }) => <SortableCell cellName="ID" column={column} />,
    cell: ({ row }) => (
      <CopyTextToClipboard
        textToCopy={row.original.containerId}
        transform={() => row.original.containerId?.slice(0, 12)}
        groupClassName="rowid"
      />
    ),
  },
  {
    accessorKey: 'image',
    header: ({ column }) => <SortableCell cellName="Image" column={column} />,
    cell: ({ row }) => <ImageName image={row.original.imageView ?? undefined} />,
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
    cell: ({ row }) => <div>{row.original.stack && truncate(row.original.stack, 10, 'left')}</div>,
    sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
      return rowA.original.stack < rowB.original.stack ? 1 : -1;
    },
  },
  {
    id: 'actions',
    cell: ({ row }) => <ActionsCell container={row.original} />,
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

const ActionsCell = ({ container }: { container: ContainerView }) => {
  const { setDialogData } = useContainersContext();
  return <TableDropdown container={container} setDialogData={setDialogData} />;
};

export const ContainersTable = () => {
  const { containers, isLoading, setSelectedRows } = useContainersContext();
  return (
    <div className="flex flex-col gap-3">
      <DataTable
        columns={columns}
        data={containers ?? []}
        isLoading={isLoading}
        onSelectionChange={(ids: string[]) => setSelectedRows(containers?.filter((c) => ids.includes(c.id!)))}
      />
      <div className="text-muted-foreground text-xs p-2 font-normal">
        {containers && containers.length > 0 && (
          <span>
            Showing {containers?.length} of {containers?.length} container(s)
          </span>
        )}
      </div>
    </div>
  );
};
