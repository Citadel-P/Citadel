import { DataTable } from '@/components/ui/data-table';
import { ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { useContainersContext } from './ContainersContext';
import { ContainerView, ContainerStateStatus, ContainerStatView, PortView } from '@/api/_generated';
import { truncate } from '@/lib/truncate';
import { toFixedNumber } from '@/lib/utils';
import { byteTransform } from '@/lib/bytes.helper';
import SortableCell from '@/components/ui/SortableCell';
import { Link } from 'react-router';
import { ContainerDropdownActions } from './ContainerDropdownActions';
import { fromNow } from '@/lib/dayjs.helper';
import { ContainerStateIndicator } from './ContainerStateIndicator';

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
      <div className="flex items-center whitespace-nowrap">
        <div className="flex items-center">
          <ContainerStateIndicator stat={row.original.state ?? ContainerStateStatus.Exited} />
        </div>
        <div>
          <div className="mb-1 text-[13px] font-semibold text-foreground">
            <Link to={`../containers/${row.original.containerId?.slice(0, 12)}/logs`} className="hover:underline">
              {row.original.name ? row.original.name?.slice(1) : ''}
            </Link>
          </div>
          <div className="text-muted-foreground/50 text-xs">{row.original.containerId?.slice(0, 12)}</div>
        </div>
      </div>
    ),
    sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
      return rowA.original.name < rowB.original.name ? 1 : -1;
    },
  },
  {
    accessorKey: 'image',
    header: ({ column }) => <SortableCell cellName="Image" column={column} />,
    cell: ({ row }) => <div className="text-[13px]">{truncate(row.original.image ?? '', 24)}</div>,
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
    cell: ({ row }) =>
      (row.original.ports as PortView[])?.map((port: PortView, i) => (
        <div key={i} className="text-xs">
          <span>
            {port.publicPort !== undefined && port.publicPort !== null && (port.publicPort as number) > 0 && (
              <span>{port.publicPort + ':' + port.privatePort}</span>
            )}
          </span>
        </div>
      )),
  },
  {
    accessorKey: 'status',
    header: ({ column }) => <SortableCell cellName="Updated" column={column} />,
    cell: ({ row }) => <div className="text-[12px]">{fromNow(new Date(row.original.updated).getTime())}</div>,
  },
  {
    accessorKey: 'stack',
    header: ({ column }) => <SortableCell cellName="Stack" column={column} />,
    cell: ({ row }) => <div className="text-xs">{row.original.stack && truncate(row.original.stack, 10, 'left')}</div>,
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
    return <div className="text-xs text-muted">0B / 0B</div>;
  }
  return (
    <div className="text-xs">
      <span>
        {byteTransform(container.lastStats?.memoryActive ?? 0, 2) +
          ' / ' +
          byteTransform(container.lastStats?.memoryLimit ?? 0, 2)}
      </span>
    </div>
  );
};

const CPUCell = ({ container }: { container: ContainerView }) => {
  if (container.state !== ContainerStateStatus.Running) {
    return <div className="text-xs text-muted">0%</div>;
  }
  return (
    <div className="text-xs">
      {container.lastStats?.cpuUsage ? toFixedNumber(container.lastStats?.cpuUsage, 'percent') : '0%'}
    </div>
  );
};

const ActionsCell = ({ container }: { container: ContainerView }) => {
  const { setDialogData } = useContainersContext();
  return (
    <div className="text-center">
      <ContainerDropdownActions container={container} setDialogData={setDialogData} />
    </div>
  );
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
      <div className="text-muted-foreground text-xs font-normal ">
        {containers && containers.length > 0 && (
          <span>
            Showing {containers?.length} of {containers?.length} container(s)
          </span>
        )}
      </div>
    </div>
  );
};
