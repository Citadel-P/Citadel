import { IpAddressManagementConfig, IpamSubnetConfiguration } from '@/api/_generated';
import { DataTable } from '@/components/ui/data-table';
import { ColumnDef } from '@tanstack/react-table';
import { useMemo } from 'react';

const columns: ColumnDef<IdentifiableIpamSubnetConfiguration>[] = [
  {
    accessorKey: 'subnet',
    header: () => <span>Subnet</span>,
    cell: ({ row }) => <span>{row.original.subnet}</span>,
  },
  {
    accessorKey: 'gateway',
    header: () => <span>Gateway</span>,
    cell: ({ row }) => <span>{row.original.gateway}</span>,
  },
  {
    accessorKey: 'ipRange',
    header: () => <span>IpRange</span>,
    cell: ({ row }) => <span>{row.original.ipRange}</span>,
  },
];

export const IPAMInfoTable = ({ ipam }: { ipam: IpAddressManagementConfig | undefined }) => {
  const config: IdentifiableIpamSubnetConfiguration[] | undefined = useMemo(
    () =>
      ipam?.config.map((config, id) => ({
        id: `${id}-${ipam.driver}`,
        ...config,
      })),
    [ipam],
  );
  if (!ipam) return <></>;

  return (
    <div className="flex flex-col gap-3">
      <DataTable columns={columns} data={config ?? []} isLoading={false} />
    </div>
  );
};
type IdentifiableIpamSubnetConfiguration = IpamSubnetConfiguration & { id: string };
