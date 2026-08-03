import { DataTable } from '@/components/ui/data-table';
import { DockerNetworkResultView } from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { truncate } from '@/lib/truncate';
import { useMemo } from 'react';
import { useNavigate, useParams } from 'react-router';
import { formatId } from '@/lib/utils';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useSelectedResources } from '@/lib/atoms';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { ActionData } from '@/pages/types';
import { SystemBadge } from '@/components/custom/system-badge';

export const NetworksTable = ({
  items,
  actions,
  isLoading,
}: {
  isLoading: boolean;
  items: DockerNetworkResultView[];
  actions: Record<
    string,
    React.FC<{ resource: DockerNetworkResultView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) => {
  const [_, setSelectedResources] = useSelectedResources<DockerNetworkResultView>('Network');
  const cols = useMemo(() => columns(actions ?? {}), [actions]);

  return <DataTable columns={cols} data={items} isLoading={isLoading} onSelectionChange={setSelectedResources} />;
};

const columns = (
  actions: Record<
    string,
    React.FC<{ resource: DockerNetworkResultView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >,
): ColumnDef<DockerNetworkResultView>[] => [
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
        aria-label="Select network"
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'name',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => <NetworkNameRow network={row.original} />,
    sortingFn: (rowA: any, rowB: any): number => rowA.original?.name?.localeCompare(rowB.original?.name),
  },
  {
    accessorKey: 'driver',
    header: ({ column }) => <SortableCell cellName="Driver" column={column} />,
    cell: ({ row }) => <span className="">{row.original.driver}</span>,
    sortingFn: (rowA, rowB) => (rowA.original.driver! < rowB.original.driver! ? 1 : -1),
  },
  {
    accessorKey: 'attachable',
    header: ({ column }) => <SortableCell cellName="Attachable" column={column} />,
    cell: ({ row }) => <span className="">{row.original.attachable ? 'true' : 'false'}</span>,
    sortingFn: (rowA, rowB) => (rowA.original.attachable! < rowB.original.attachable! ? 1 : -1),
  },
  {
    accessorKey: 'ipam.driver',
    header: ({ column }) => <SortableCell cellName="IPAM Driver" column={column} />,
    cell: ({ row }) => <span className="">{row.original.ipam?.driver}</span>,
    sortingFn: (rowA, rowB) => ((rowA.original.ipam?.driver ?? '') < (rowB.original.ipam?.driver ?? '') ? 1 : -1),
  },
  {
    accessorKey: 'ipam.config.subnet.ipv4',
    header: ({ column }) => <SortableCell cellName="Subnet (IPv4)" column={column} />,
    cell: ({ row }) => {
      const configs = row.original.ipam?.config ?? [];
      const ipv4 = configs.find((cfg) => cfg.gateway && /^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$/.test(cfg.gateway));
      return <span className="">{ipv4?.subnet ?? '-'}</span>;
    },
  },
  {
    accessorKey: 'ipam.config.gateway.ipv4',
    header: ({ column }) => <SortableCell cellName="Gateway (IPv4)" column={column} />,
    cell: ({ row }) => {
      const configs = row.original.ipam?.config ?? [];
      const ipv4 = configs.find((cfg) => cfg.gateway && /^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$/.test(cfg.gateway));
      return <span className="">{ipv4?.gateway ?? '-'}</span>;
    },
  },
  {
    accessorKey: 'ipam.config.ipRange.ipv4',
    header: ({ column }) => <SortableCell cellName="IP Range (IPv4)" column={column} />,
    cell: ({ row }) => {
      const configs = row.original.ipam?.config ?? [];
      const ipv4 = configs.find((cfg) => cfg.gateway && /^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$/.test(cfg.gateway));
      return <span className="">{ipv4?.ipRange ?? '-'}</span>;
    },
  },
  {
    accessorKey: 'ipam.config.subnet.ipv6',
    header: ({ column }) => <SortableCell cellName="Subnet (IPv6)" column={column} />,
    cell: ({ row }) => {
      const configs = row.original.ipam?.config ?? [];
      const ipv6 = configs.find((cfg) => cfg.gateway && cfg.gateway.includes(':'));
      return <span className="">{ipv6?.subnet ?? '-'}</span>;
    },
  },
  {
    accessorKey: 'ipam.config.gateway.ipv6',
    header: ({ column }) => <SortableCell cellName="Gateway (IPv6)" column={column} />,
    cell: ({ row }) => {
      const configs = row.original.ipam?.config ?? [];
      const ipv6 = configs.find((cfg) => cfg.gateway && cfg.gateway.includes(':'));
      return <span className="">{ipv6?.gateway ?? '-'}</span>;
    },
  },
  {
    accessorKey: 'ipam.config.ipRange.ipv6',
    header: ({ column }) => <SortableCell cellName="IP Range (IPv6)" column={column} />,
    cell: ({ row }) => {
      const configs = row.original.ipam?.config ?? [];
      const ipv6 = configs.find((cfg) => cfg.gateway && cfg.gateway.includes(':'));
      return <span className="">{ipv6?.ipRange ?? '-'}</span>;
    },
  },
  {
    id: 'actions',
    cell: ({ row }) => <RowActionMenu resource={row.original} actions={actions} />,
  },
];

export const NetworkNameRow = ({ network }: { network: DockerNetworkResultView }) => {
  const { platformId } = useParams<{ platformId: string }>();
  const navigate = useNavigate();
  function onClick() {
    navigate(`/platforms/${platformId}/networks/${formatId(network.id)}/`);
  }

  return (
    <div className="flex items-center gap-2 whitespace-nowrap">
      <div className="flex items-center">
        <StateIndicator value={network.inUse} />
      </div>
      <span
        className="cursor-pointer hover:underline"
        onClick={onClick}
        title={network.name}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            onClick();
          }
        }}
        tabIndex={0}
        role="button"
        aria-label="Show network details">
        {truncate(network.name ?? '', 32, 'right')}
      </span>
      {network.isSystem && <SystemBadge description="Docker system network" />}
    </div>
  );
};
