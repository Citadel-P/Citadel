import { useMemo } from 'react';
import type { SwarmServiceView } from '@/api/generated/api.types';
import { DataTable } from '@/components/ui/data-table';
import type { SwarmInventoryUpdate } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useRead } from '@/lib/hooks';
import type { ColumnDef } from '@tanstack/react-table';
import { Link } from 'react-router';
import { useLiveSwarmItems } from './hooks/useSwarmResourceGroup';

export type SwarmResourceWithReferences = {
  platformId: string;
  serviceNames: string[];
};

const selectServices = (inventory: SwarmInventoryUpdate) => inventory.services.items;

const useReferencedServices = (resource: SwarmResourceWithReferences) => {
  const args = useMemo(() => ({ platformId: resource.platformId }), [resource.platformId]);
  const query = useRead('listSwarmServices', args);
  const services = useLiveSwarmItems(resource.platformId, 'listSwarmServices', args, query, selectServices);
  const names = useMemo(() => new Set(resource.serviceNames), [resource.serviceNames]);
  const items = useMemo(() => services.filter((service) => names.has(service.name)), [names, services]);
  return { items, isLoading: query.isLoading };
};

export const ReferencingServices = ({ resource }: { resource: SwarmResourceWithReferences }) => {
  const { items, isLoading } = useReferencedServices(resource);
  const columns = useMemo<ColumnDef<SwarmServiceView>[]>(
    () => [
      {
        accessorKey: 'name',
        header: 'Name',
        cell: ({ row }) => (
          <Link className="table-link" to={`/platforms/${resource.platformId}/services/${row.original.id}`}>
            {row.original.name}
          </Link>
        ),
      },
      { accessorKey: 'mode', header: 'Mode' },
      {
        id: 'replicas',
        header: 'Replicas',
        cell: ({ row }) => `${row.original.runningTaskCount}/${row.original.desiredTaskCount}`,
      },
      { accessorKey: 'image', header: 'Image' },
    ],
    [resource.platformId],
  );

  return <DataTable columns={columns} data={items} isLoading={isLoading} />;
};
