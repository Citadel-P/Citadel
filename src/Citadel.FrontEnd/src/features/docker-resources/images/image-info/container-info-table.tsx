import { ContainerImageResult, ContainerStateStatus, InspectImageView, PlatformView } from '@/api/generated/api.types';
import { useAppContext } from '@/lib/context/app-context';
import { StateIndicator } from '@/components/custom/state-indicator';
import { DataTable } from '@/components/ui/data-table';
import { PortsDisplay } from '@/components/ui/PortsDisplay';
import { truncate } from '@/lib/truncate';
import { formatId } from '@/lib/utils';
import { ColumnDef } from '@tanstack/react-table';
import { Database, Network } from 'lucide-react';
import { Link } from 'react-router';

const columns = (currentPlatform: PlatformView | undefined): ColumnDef<ContainerImageResult>[] => [
  {
    accessorKey: 'name',
    header: () => <span>Name</span>,
    cell: ({ row }) => (
      <div className="flex flex-wrap gap-2 text-[13px] items-center">
        <StateIndicator value={row.original.state ?? ContainerStateStatus.Exited} />
        <Link to={`/containers/${row.original.id?.slice(0, 12)}`} className="table-link">
          {row.original.name?.slice(1)}
        </Link>
      </div>
    ),
  },
  {
    accessorKey: 'networks',
    header: () => <span>Networks</span>,
    cell: ({ row }) => (
      <div className="text-foreground gap-2 flex flex-wrap items-center">
        <Network width={13} height={13} className="text-primary" />
        {Object.entries(row.original.networks).map(([name, id]) => (
          <Link to={`/platforms/${currentPlatform?.id}/networks/${formatId(id)}`} key={id} className="table-link">
            {name}
          </Link>
        ))}
      </div>
    ),
  },
  {
    accessorKey: 'volumes',
    header: () => <span>Volumes</span>,
    cell: ({ row }) => (
      <div className="text-foreground gap-2 flex flex-wrap items-center">
        {row.original.volumes.length > 0 && <Database width={13} height={13} className="text-primary" />}
        {row.original.volumes.map((volume) => (
          <Link to={`/platforms/${currentPlatform?.id}/volumes/${volume}`} key={volume} className="table-link">
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
];

export const ContainerInfoTable = ({ image }: { image: InspectImageView | undefined }) => {
  const { currentPlatform } = useAppContext();
  if (!image) return <></>;
  return (
    <div className="flex flex-col gap-3">
      <DataTable columns={columns(currentPlatform)} data={image.containers} isLoading={false} />
    </div>
  );
};
