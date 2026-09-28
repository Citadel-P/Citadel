import { ContainerImageResult, InspectImageView, PlatformView } from '@/api/generated/api.types';
import { useAppContext } from '@/lib/context/app-context';
import { DataTable } from '@/components/ui/data-table';
import { PortsDisplay } from '@/components/custom/ports-display';
import { ColumnDef } from '@tanstack/react-table';
import { DockerContainerCell, DockerNetworksCell, DockerVolumesCell } from '@/components/custom/common';

const columns = (currentPlatform: PlatformView | undefined): ColumnDef<ContainerImageResult>[] => [
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
    accessorKey: 'networks',
    header: () => <span>Networks</span>,
    cell: ({ row }) => <DockerNetworksCell networks={row.original.networks} platformId={currentPlatform?.id} />,
  },
  {
    accessorKey: 'volumes',
    header: () => <span>Volumes</span>,
    cell: ({ row }) => <DockerVolumesCell volumes={row.original.volumes} platformId={currentPlatform?.id} />,
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
    <div className="min-w-0">
      <DataTable columns={columns(currentPlatform)} data={image.containers} isLoading={false} />
    </div>
  );
};
