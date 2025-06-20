import { DataTable } from '@/components/ui/data-table';
import { DockerNetworkResult } from '@/api/_generated';
import SortableCell from '@/components/ui/SortableCell';
import { ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { useContextSelector } from 'use-context-selector';
import { truncate } from '@/lib/truncate';
import { useEffect, useCallback, useMemo, memo } from 'react';
import { AppContext } from '@/AppProvider';
import { useGETNetworks } from './hooks/useGETNetworks';
import { NetworksContext } from './NetworksProvider';
import DropdownTableMenu from './DropdownTableMenu';
import { DeleteNetworkDialog } from './dialogs/DeleteNetworkDialog';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { NetworkInspectSheet } from './NetworkInspectSheet';

export default function NetworksTable() {
  const currentPlatform = useContextSelector(AppContext, (v) => v?.currentPlatform);
  const { data, isLoading, isSuccess } = useGETNetworks(currentPlatform?.id);
  const setSelectedRows = useContextSelector(NetworksContext, (v) => v?.setSelectedRows)!;
  const setNetworks = useContextSelector(NetworksContext, (v) => v?.setNetworks)!;
  const networks = useContextSelector(NetworksContext, (v) => v?.networks);
  const setSheetOpen = useContextSelector(NetworksContext, (v) => v?.setSheetOpen)!;
  const setCurrentNetwork = useContextSelector(NetworksContext, (v) => v?.setCurrentNetwork)!;
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

  const handleShowSheet = (network: DockerNetworkResult) => {
    setCurrentNetwork(network);
    setSheetOpen(true);
  };

  return (
    <>
      <div className="flex flex-col gap-3">
        <DataTable
          columns={columns(handleShowSheet)}
          data={networks ?? []}
          isLoading={isLoading}
          onSelectionChange={handleSelectionChange}
        />
        <div className="text-muted-foreground text-xs font-normal">
          {rowCount > 0 && (
            <span>
              Showing {rowCount} of {rowCount} network(s)
            </span>
          )}
        </div>
      </div>
      <DeleteNetworkDialog />
      <NetworkInspectSheet />
    </>
  );
}

const columns = (handleShowSheet: (network: DockerNetworkResult) => void): ColumnDef<DockerNetworkResult>[] => [
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
    cell: ({ row }) => <NetworkNameRow network={row.original} onShowSheet={handleShowSheet} />,
    sortingFn: (rowA: any, rowB: any): number => rowA.original?.name?.localeCompare(rowB.original?.name),
  },
  {
    accessorKey: 'driver',
    header: ({ column }) => <SortableCell cellName="Driver" column={column} />,
    cell: ({ row }) => <span className="text-[13px]">{row.original.driver}</span>,
    sortingFn: (rowA, rowB) => (rowA.original.driver! < rowB.original.driver! ? 1 : -1),
  },
  {
    accessorKey: 'attachable',
    header: ({ column }) => <SortableCell cellName="Attachable" column={column} />,
    cell: ({ row }) => <span className="text-[13px]">{row.original.attachable ? 'true' : 'false'}</span>,
    sortingFn: (rowA, rowB) => (rowA.original.attachable! < rowB.original.attachable! ? 1 : -1),
  },
  {
    accessorKey: 'ipam.driver',
    header: ({ column }) => <SortableCell cellName="IPAM Driver" column={column} />,
    cell: ({ row }) => <span className="text-[13px]">{row.original.ipam?.driver}</span>,
    sortingFn: (rowA, rowB) => ((rowA.original.ipam?.driver ?? '') < (rowB.original.ipam?.driver ?? '') ? 1 : -1),
  },
  {
    accessorKey: 'ipam.config.subnet.ipv4',
    header: ({ column }) => <SortableCell cellName="Subnet (IPv4)" column={column} />,
    cell: ({ row }) => {
      const configs = row.original.ipam?.config ?? [];
      const ipv4 = configs.find((cfg) => cfg.gateway && /^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$/.test(cfg.gateway));
      return <span className="text-[13px]">{ipv4?.subnet ?? '-'}</span>;
    },
  },
  {
    accessorKey: 'ipam.config.gateway.ipv4',
    header: ({ column }) => <SortableCell cellName="Gateway (IPv4)" column={column} />,
    cell: ({ row }) => {
      const configs = row.original.ipam?.config ?? [];
      const ipv4 = configs.find((cfg) => cfg.gateway && /^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$/.test(cfg.gateway));
      return <span className="text-[13px]">{ipv4?.gateway ?? '-'}</span>;
    },
  },
  {
    accessorKey: 'ipam.config.ipRange.ipv4',
    header: ({ column }) => <SortableCell cellName="IP Range (IPv4)" column={column} />,
    cell: ({ row }) => {
      const configs = row.original.ipam?.config ?? [];
      const ipv4 = configs.find((cfg) => cfg.gateway && /^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$/.test(cfg.gateway));
      return <span className="text-[13px]">{ipv4?.ipRange ?? '-'}</span>;
    },
  },
  {
    accessorKey: 'ipam.config.subnet.ipv6',
    header: ({ column }) => <SortableCell cellName="Subnet (IPv6)" column={column} />,
    cell: ({ row }) => {
      const configs = row.original.ipam?.config ?? [];
      const ipv6 = configs.find((cfg) => cfg.gateway && cfg.gateway.includes(':'));
      return <span className="text-[13px]">{ipv6?.subnet ?? '-'}</span>;
    },
  },
  {
    accessorKey: 'ipam.config.gateway.ipv6',
    header: ({ column }) => <SortableCell cellName="Gateway (IPv6)" column={column} />,
    cell: ({ row }) => {
      const configs = row.original.ipam?.config ?? [];
      const ipv6 = configs.find((cfg) => cfg.gateway && cfg.gateway.includes(':'));
      return <span className="text-[13px]">{ipv6?.gateway ?? '-'}</span>;
    },
  },
  {
    accessorKey: 'ipam.config.ipRange.ipv6',
    header: ({ column }) => <SortableCell cellName="IP Range (IPv6)" column={column} />,
    cell: ({ row }) => {
      const configs = row.original.ipam?.config ?? [];
      const ipv6 = configs.find((cfg) => cfg.gateway && cfg.gateway.includes(':'));
      return <span className="text-[13px]">{ipv6?.ipRange ?? '-'}</span>;
    },
  },
  {
    id: 'actions',
    cell: ({ row }) => (
      <div className="text-center">
        <DropdownTableMenu network={row.original} />
      </div>
    ),
  },
];

const NetworkNameRow = ({
  network,
  onShowSheet,
}: {
  network: DockerNetworkResult;
  onShowSheet: (network: DockerNetworkResult) => void;
}) => {
  return (
    <div className="flex items-center whitespace-nowrap">
      <div className="flex items-center">
        <NetworkStatusTooltip inUse={network.inUse ?? false} />
      </div>
      <span
        className="cursor-pointer hover:underline"
        onClick={() => onShowSheet(network)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            onShowSheet(network);
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

const NetworkStatusTooltip = memo(({ inUse }: { inUse: boolean }) => {
  const getStatusClass = () => (inUse ? 'bg-green-500' : 'bg-gray-500');

  return (
    <TooltipProvider delayDuration={200}>
      <Tooltip>
        <TooltipTrigger asChild>
          <div className={`${getStatusClass()} mr-2 h-2 w-2 rounded-full`} />
        </TooltipTrigger>
        <TooltipContent>
          <span>{inUse ? 'In use' : 'Unused'}</span>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
});

NetworkStatusTooltip.displayName = 'NetworkStatusTooltip';
