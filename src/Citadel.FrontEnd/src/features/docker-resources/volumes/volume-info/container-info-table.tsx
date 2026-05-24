import { ContainerVolumeResult, DockerVolumeResultView, PlatformView } from '@/api/generated/api.types';
import { useAppContext } from '@/lib/context/app-context';
import { DataTable } from '@/components/ui/data-table';
import { PortsDisplay } from '@/components/custom/ports-display';
import { truncate } from '@/lib/truncate';
import { formatId } from '@/lib/utils';
import { ColumnDef } from '@tanstack/react-table';
import { Link } from 'react-router';
import { DockerContainerCell, DockerImageCell, DockerNetworksCell } from '@/components/custom/common';

const columns = (currentPlatform: PlatformView | undefined): ColumnDef<ContainerVolumeResult>[] => [
  {
    accessorKey: 'name',
    header: () => <span>Name</span>,
    cell: ({ row }) => (
      <DockerContainerCell
        name={row.original.name}
        id={row.original.id}
        state={row.original.state}
        platformId={currentPlatform?.id ?? ''}
      />
    ),
  },
  {
    accessorKey: 'image',
    header: () => <span>Image</span>,
    cell: ({ row }) => (
      <DockerImageCell>
        <Link
          to={`/platforms/${currentPlatform?.id}/images/${formatId(row.original.imageId)}`}
          title={row.original.image}
          className="table-link">
          {truncate(row.original.image.replace('sha256:', ''), 24)}
        </Link>
      </DockerImageCell>
    ),
  },
  {
    accessorKey: 'networks',
    header: () => <span>Networks</span>,
    cell: ({ row }) => <DockerNetworksCell networks={row.original.networks} platformId={currentPlatform?.id} />,
  },

  {
    accessorKey: 'ports',
    header: () => <span>Ports</span>,
    cell: ({ row }) => <PortsDisplay ports={row.original.ports} />,
  },
];

export const ContainerInfoTable = ({ volume }: { volume: DockerVolumeResultView | undefined }) => {
  const { currentPlatform } = useAppContext();
  if (!volume) return <></>;
  return (
    <div className="rounded-sm border p-1 shadow-xs">
      <DataTable columns={columns(currentPlatform)} data={volume.containers} isLoading={false} />
    </div>
  );
};
