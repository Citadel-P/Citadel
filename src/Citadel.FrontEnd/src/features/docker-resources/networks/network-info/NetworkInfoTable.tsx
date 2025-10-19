import { DockerNetworkDetails } from '@/api/generated/api.types';
import { DataTable } from '@/components/ui/data-table';
import { ColumnDef } from '@tanstack/react-table';

const columns: ColumnDef<DockerNetworkDetails>[] = [
  {
    accessorKey: 'driver',
    header: () => <span>Driver</span>,
    cell: ({ row }) => <span>{row.original.driver}</span>,
  },
  {
    accessorKey: 'scope',
    header: () => <span>Scope</span>,
    cell: ({ row }) => <span>{row.original.scope}</span>,
  },
  {
    accessorKey: 'attachable',
    header: () => <span>Attachable</span>,
    cell: ({ row }) => <span>{row.original.attachable ? 'true' : 'false'}</span>,
  },
  {
    accessorKey: 'internal',
    header: () => <span>Internal</span>,
    cell: ({ row }) => <span>{row.original.internal ? 'true' : 'false'}</span>,
  },
  {
    accessorKey: 'ingress',
    header: () => <span>Ingress</span>,
    cell: ({ row }) => <span>{row.original.ingress ? 'true' : 'false'}</span>,
  },
];

export const NetworkInfoTable = ({ network }: { network: DockerNetworkDetails | undefined }) => {
  if (!network) return <></>;
  return (
    <div className="flex flex-col gap-3">
      <DataTable columns={columns} data={network ? [{ ...network }] : []} isLoading={false} />
    </div>
  );
};
