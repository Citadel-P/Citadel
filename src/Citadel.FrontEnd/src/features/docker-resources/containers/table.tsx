import { DataTable } from '@/components/ui/data-table';
import { ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import {
  ContainerView,
  ContainerStateStatus,
  ContainerStatView,
  ResourceControlState,
} from '@/api/generated/api.types';
import { truncate } from '@/lib/truncate';
import { formatId, isUnmanagedContainer } from '@/lib/utils';
import SortableCell from '@/components/custom/sortable-cell';
import { Link } from 'react-router';
import { CopyToClipboard } from '@/components/custom/copy-to-clipboard';
import { PortsDisplay } from '@/components/custom/ports-display';
import { ImageName } from './container-info/';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useMemo } from 'react';
import { useSelectedResources } from '@/lib/atoms';
import { ActionData } from '@/pages/types';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { ContentCard } from '@/components/custom/content-card';
import { CPUCell, MemoryUsageCell } from '@/components/custom/common';
import { Unlink } from 'lucide-react';

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
  const [_, setSelectedResources] = useSelectedResources<ContainerView>('Container');
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
      <div className="flex items-center gap-2">
        <StateIndicator
          value={row.original.state ?? ContainerStateStatus.Exited}
          isProcessing={row.original.controlState === ResourceControlState.Processing}
          kind="container"
        />
        <Link to={`./${formatId(row.original.containerId)}`} className="table-link truncate" title={row.original.name}>
          {row.original.name ? truncate(row.original.name?.slice(1), 24) : ''}
        </Link>
        {isUnmanagedContainer(row.original) && (
          <span
            className="inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-sm text-amber-500"
            title="Unmanaged container">
            <Unlink className="h-3 w-3" />
            <span className="sr-only">Unmanaged container</span>
          </span>
        )}
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
    cell: ({ row }) => <CPUCell state={row.original.state} stats={row.original.lastStats} />,
    sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
      if (!rowA.original.stats || !rowB.original.stats) return 0;
      return rowA.original.stats[0]?.cpuUsage < rowB.original.stats[0]?.cpuUsage ? 1 : -1;
    },
  },
  {
    accessorKey: 'memory',
    header: ({ column }) => <SortableCell cellName="Memory" column={column} />,
    cell: ({ row }) => <MemoryUsageCell state={row.original.state} stats={row.original.lastStats} />,
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
