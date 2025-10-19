import { DockerNetworkDetails, NetworkConnectedContainer, PlatformView } from '@/api/generated/api.types';
import { useAppContext } from '@/AppContext';
import { DataTable } from '@/components/ui/data-table';
import { ColumnDef } from '@tanstack/react-table';
import { Box } from 'lucide-react';
import { useMemo } from 'react';
import { Link } from 'react-router';

const columns = (currentPlatform: PlatformView | undefined): ColumnDef<NetworkConnectedContainerProps>[] => [
  {
    accessorKey: 'name',
    header: () => <span>Name</span>,
    cell: ({ row }) => (
      <div className="flex flex-wrap gap-2 text-[13px] items-center">
        <Box width={13} height={13} className="text-primary" />
        <Link to={`/containers/${row.original.id?.slice(0, 12)}`} className="table-link">
          {row.original.name}
        </Link>
      </div>
    ),
  },
  {
    accessorKey: 'ipv4',
    header: () => <span>IPv4</span>,
    cell: ({ row }) => (
      <div className="text-foreground gap-2 flex flex-wrap items-center">{row.original.ipV4Address}</div>
    ),
  },
  {
    accessorKey: 'ipv6',
    header: () => <span>IPv6</span>,
    cell: ({ row }) => (
      <div className="text-foreground gap-2 flex flex-wrap items-center">{row.original.ipv6Address}</div>
    ),
  },
  {
    accessorKey: 'macAddress',
    header: () => <span>Mac</span>,
    cell: ({ row }) => (
      <div className="text-foreground gap-2 flex flex-wrap items-center">{row.original.macAddress}</div>
    ),
  },
];

export const ContainerInfoTable = ({ network }: { network: DockerNetworkDetails | undefined }) => {
  const { currentPlatform } = useAppContext();
  const containers: NetworkConnectedContainerProps[] = useMemo(
    () =>
      Object.entries(network?.containers ?? [])
        .map(([id, container]) => ({ id, ...container }))
        .sort((a, b) => {
          const aIpv4 = a.ipV4Address || '';
          const bIpv4 = b.ipV4Address || '';
          const ipv4Compare = aIpv4.localeCompare(bIpv4);
          if (ipv4Compare !== 0) {
            return ipv4Compare;
          }
          const aIpv6 = a.ipv6Address || '';
          const bIpv6 = b.ipv6Address || '';
          return aIpv6.localeCompare(bIpv6);
        }),
    [network?.containers],
  );
  if (!network) return <></>;
  return (
    <div className="flex flex-col gap-3">
      <DataTable columns={columns(currentPlatform)} data={containers} isLoading={false} />
    </div>
  );
};

type NetworkConnectedContainerProps = NetworkConnectedContainer & { id: string };
