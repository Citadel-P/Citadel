import { DataTable } from '@/components/ui/data-table';
import { DockerNetworkResult } from '@/api/generated/api.types';
import SortableCell from '@/components/ui/SortableCell';
import { ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { truncate } from '@/lib/truncate';
import { useEffect, useCallback, useMemo } from 'react';
import { useNetworksContext } from './NetworksContext';
import { useAppContext } from '@/AppContext';
import { TableDropDown } from './table-dropdown';
import { DeleteDialog } from './delete-dialog';
import { useNavigate, useParams } from 'react-router';
import { formatId } from '@/lib/utils';
import { useRead } from '@/lib/hooks';
import { StateIndicator } from '@/components/custom/state-indicator';

export default function NetworksTable() {
  const { currentPlatform } = useAppContext();
  const { data, isLoading, isSuccess } = useRead('listNetworks', { platformId: currentPlatform?.id });
  const { setSelectedRows, setNetworks, networks, dialogData, setDialogData, requestDelete, deleteIsPending } =
    useNetworksContext();

  // Update networks when data is fetched
  useEffect(() => {
    if (isSuccess && data?.data.networks) {
      setNetworks(data.data.networks);
    }
    return () => {
      setNetworks([]);
      setSelectedRows([]);
    };
  }, [data, isSuccess, setNetworks, setSelectedRows]);

  // Memoized selection change handler
  const handleSelectionChange = useCallback(
    (ids: string[]) => {
      setSelectedRows(networks?.filter((c) => ids.includes(c.id!)));
    },
    [networks, setSelectedRows],
  );

  // Memoized row count
  const rowCount = useMemo(() => networks?.length ?? 0, [networks]);

  return (
    <div className="flex flex-col gap-3">
      <DataTable
        columns={columns}
        data={networks ?? []}
        isLoading={isLoading}
        onSelectionChange={handleSelectionChange}
      />
      <div className="text-muted-foreground text-xs p-2 font-normal">
        {rowCount > 0 && (
          <span>
            Showing {rowCount} of {rowCount} network(s)
          </span>
        )}
      </div>
      <DeleteDialog
        dialogData={dialogData}
        setDialogData={setDialogData}
        requestDelete={requestDelete}
        deleteIsPending={deleteIsPending}
      />
    </div>
  );
}

const columns: ColumnDef<DockerNetworkResult>[] = [
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
    cell: ({ row }) => <TableDropDown network={row.original} />,
  },
];

const NetworkNameRow = ({ network }: { network: DockerNetworkResult }) => {
  const { platformId } = useParams<{ platformId: string }>();
  const navigate = useNavigate();
  function onClick() {
    navigate(`/platforms/${platformId}/networks/${formatId(network.id)}/`);
  }

  return (
    <div className="flex items-center whitespace-nowrap">
      <div className="flex items-center">
        <StateIndicator value={network.inUse} />
      </div>
      <span
        className="cursor-pointer hover:underline text-[13px]"
        onClick={onClick}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            onClick();
          }
        }}
        tabIndex={0}
        role="button"
        aria-label="Show network details">
        {truncate(network.name ?? '', 35, 'right')}
      </span>
    </div>
  );
};
