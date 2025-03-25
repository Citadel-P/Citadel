import { DataTable } from '@/components/ui/data-table';
import { ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { useContextSelector } from 'use-context-selector';
import { ContainersContext } from './ContainersProvider';
import { ContainerInfoView, PortView } from '@/api/_generated';
import { truncate } from '@/lib/truncate';
import { toFixedNumber } from '@/lib/utils';
import { byteTransform } from '@/lib/bytes.helper';
import SortableCell from '@/components/ui/SortableCell';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { Link } from 'react-router';
import DropdownTableMenu from './DropdownTableMenu';

const columns: ColumnDef<ContainerInfoView>[] = [
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
          <ContainerStatTootltip stat={row.original.state ?? 'exited'} />
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
    cell: ({ row }) => (
      <div className="text-xs">
        {(row.original as ContainerInfoView).stats &&
          toFixedNumber((row.original as ContainerInfoView).stats?.at(0)?.cpuUsage, 'percent')}
      </div>
    ),
    sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
      if (!rowA.original.stats || !rowB.original.stats) return 0;
      return rowA.original.stats[0]?.cpuUsage < rowB.original.stats[0]?.cpuUsage ? 1 : -1;
    },
  },
  {
    accessorKey: 'memory',
    header: ({ column }) => <SortableCell cellName="Memory" column={column} />,
    cell: ({ row }) => (
      <div className="text-xs">
        {(row.original as ContainerInfoView)?.stats?.at(0)?.memoryUsage && (
          <span>
            {byteTransform((row.original as ContainerInfoView).stats?.at(0)?.memoryUsage ?? 0, 2) +
              ' / ' +
              byteTransform((row.original as ContainerInfoView).stats?.at(0)?.memoryLimit ?? 0, 2)}
          </span>
        )}
      </div>
    ),
    sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
      const cA = rowA.original as ContainerInfoView;
      const cB = rowB.original as ContainerInfoView;
      if (!cA.stats?.length || !cB.stats?.length) return 0;
      if (!cA.stats[0]?.memoryUsage || !cB.stats[0]?.memoryUsage) return 0;
      return cA.stats[0]?.memoryUsage < cB.stats[0]?.memoryUsage ? 1 : -1;
    },
  },
  {
    accessorKey: 'ports',
    header: () => <span>Ports</span>,
    cell: ({ row }) =>
      (row.original.ports as PortView[])?.map((port: PortView, i) => (
        <div key={i} className="text-xs">
          <span>
            {port.publicPort !== undefined && port.publicPort !== null && port.publicPort > 0 && (
              <span>{port.publicPort + ':' + port.privatePort}</span>
            )}
          </span>
        </div>
      )),
  },
  {
    accessorKey: 'status',
    header: ({ column }) => <SortableCell cellName="Status" column={column} />,
    cell: ({ row }) => <div>{row.original?.status && truncate(row.original.status, 20)}</div>,
  },
  {
    accessorKey: 'stack',
    header: ({ column }) => <SortableCell cellName="Stack" column={column} />,
    cell: ({ row }) => (
      <div className="text-xs">
        {row.original.labels &&
          row.original.labels['com.docker.compose.project'] &&
          truncate(row.original.labels['com.docker.compose.project'], 10, 'left')}
      </div>
    ),
    sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
      if (!rowA.original.labels || !rowA.original.labels.length) return 0;
      if (!rowA.original.labels['com.docker.compose.project'] || !rowB.original.labels['com.docker.compose.project'])
        return 0;
      return rowA.original.labels['com.docker.compose.project'] < rowB.original.labels['com.docker.compose.project']
        ? 1
        : -1;
    },
  },
  {
    id: 'actions',
    cell: ({ row }) => (
      <div className="text-center">
        <DropdownTableMenu container={row.original} />
      </div>
    ),
  },
];

const ContainerStatTootltip = ({ stat }: { stat: string }) => {
  return (
    <TooltipProvider delayDuration={200}>
      <Tooltip>
        <TooltipTrigger asChild>
          <div
            className={`${
              stat === 'exited'
                ? 'bg-gray-500 mr-2 h-2 w-2 rounded-full'
                : stat === 'paused'
                  ? 'bg-orange-500 mr-2 h-2 w-2 rounded-full'
                  : stat === 'running'
                    ? 'bg-green-500 mr-2 h-2 w-2 rounded-full'
                    : stat === 'offline'
                      ? 'bg-red-500 mr-2 h-2 w-2 rounded-full'
                      : ''
            }`}
          />
        </TooltipTrigger>
        <TooltipContent>
          {stat === 'running' && <span>Running</span>}
          {stat === 'exited' && <span>Exited</span>}
          {stat === 'paused' && <span>Paused</span>}
          {stat === 'offline' && <span>Offline</span>}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
};

export const ContainersTable = () => {
  const containers = useContextSelector(ContainersContext, (v) => v?.containers) ?? [];
  const isLoading = useContextSelector(ContainersContext, (v) => v?.isLoading) ?? false;
  const setSelectedRows = useContextSelector(ContainersContext, (v) => v?.setSelectedRows)!;

  return (
    <div className="flex flex-col gap-3">
      <DataTable
        columns={columns}
        data={containers}
        isLoading={isLoading}
        onSelectionChange={(ids: string[]) => setSelectedRows(containers.filter((c) => ids.includes(c.id!)))}
      />
      <div className="text-muted-foreground text-xs font-normal ">
        {containers?.length > 0 && (
          <span>
            Showing {containers.length} of {containers.length} container(s)
          </span>
        )}
      </div>
    </div>
  );
};
