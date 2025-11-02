import { ContainerInfoView } from '@/api/generated/api.types';
import { useAppContext } from '@/lib/context/app-context';
import { DataTable } from '@/components/ui/data-table';
import { PortsDisplay } from '@/components/custom/ports-display';
import { truncate } from '@/lib/truncate';
import { formatId } from '@/lib/utils';
import { ColumnDef } from '@tanstack/react-table';
import { Clock, Database, HardDrive, Network, Server } from 'lucide-react';
import { Link } from 'react-router';
import { ImageName } from './ImageName';

const columns: ColumnDef<ContainerInfoView & { id: string | null } & { statusSnapshot: string | undefined }>[] = [
  {
    accessorKey: 'platformName',
    header: () => <span>Platform</span>,
    cell: ({ row }) => (
      <div className="flex flex-wrap gap-2  items-center">
        <Server width={12} height={12} className="text-primary" />
        <Link to={`/platforms/${row.original.platformId}`} className="table-link">
          {row.original.platformName}
        </Link>
      </div>
    ),
  },
  {
    accessorKey: 'image',
    header: () => <span>Image</span>,
    cell: ({ row }) => (
      <div className=" gap-2 flex flex-wrap items-center">
        <HardDrive width={13} height={13} className="text-primary" />
        <ImageName image={row.original.imageView ?? undefined} />
      </div>
    ),
  },
  {
    accessorKey: 'networks',
    header: () => <span>Networks</span>,
    cell: ({ row }) => (
      <div className=" text-foreground gap-2 flex flex-wrap items-center">
        <Network width={13} height={13} className="text-primary" />
        {Object.entries(row.original.networks).map(([key, value]) => (
          <Link
            to={`/platforms/${row.original.platformId}/networks/${formatId(value)}`}
            key={value}
            className="table-link">
            {key}
          </Link>
        ))}
      </div>
    ),
  },
  {
    accessorKey: 'volumes',
    header: () => <span>Volumes</span>,
    cell: ({ row }) => (
      <div className=" text-foreground gap-2 flex flex-wrap items-center">
        {row.original.volumes.length > 0 && <Database width={13} height={13} className="text-primary" />}
        {row.original.volumes.map((volume) => (
          <Link to={`/platforms/${row.original.platformId}/volumes/${volume}`} key={volume} className="table-link">
            {truncate(volume, 12)}
          </Link>
        ))}
      </div>
    ),
  },
  {
    accessorKey: 'ports',
    header: () => <span>Ports</span>,
    cell: ({ row }) => <PortsDisplay ports={row.original.ports} />,
  },
  {
    accessorKey: 'status',
    header: () => <span>Status</span>,
    cell: ({ row }) => (
      <div className=" text-foreground gap-2 flex flex-wrap items-center">
        <Clock width={13} height={13} className="text-primary" />
        {row.original.statusSnapshot}
      </div>
    ),
  },
];

export const ContainerInfoTable = ({ statusSnapshot }: { statusSnapshot: string | undefined }) => {
  const { currentContainer } = useAppContext();
  if (!currentContainer) return <></>;
  return (
    <div className="flex flex-col gap-3">
      <DataTable
        columns={columns}
        data={currentContainer ? [{ id: '1', statusSnapshot, ...currentContainer }] : []}
        isLoading={false}
      />
    </div>
  );
};
