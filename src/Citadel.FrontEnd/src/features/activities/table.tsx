import { DataTable } from '@/components/ui/data-table';
import { ActivityResourceType, ActivityView, PagedResultViewOfActivityView } from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { ColumnDef } from '@tanstack/react-table';
import { useMemo } from 'react';
import { Link } from 'react-router';
import { useSelectedResources } from '@/lib/atoms';
import { ContentCard } from '@/components/custom/content-card';
import { Cable, Layers, Rocket, Server } from 'lucide-react';
import { ActorCell, ActivityStatusCell } from '@/components/custom/common';

export const ActivitiesTable = ({
  pagedResult,
  isLoading,
}: {
  pagedResult: PagedResultViewOfActivityView;
  isLoading: boolean;
}) => {
  const [_, setSelectedResources] = useSelectedResources<ActivityView>('Deployment');
  const cols = useMemo(() => columns(), []);
  return (
    <ContentCard>
      <DataTable
        columns={cols}
        data={pagedResult?.items ?? []}
        isLoading={isLoading}
        onSelectionChange={setSelectedResources}
      />
    </ContentCard>
  );
};

const columns = (): ColumnDef<ActivityView>[] => [
  {
    accessorKey: 'eventType',
    header: ({ column }) => <SortableCell cellName="Event Type" column={column} />,
    cell: ({ row }) => row.original.eventType,
    sortingFn: (rowA: any, rowB: any): number => rowA.original?.eventType?.localeCompare(rowB.original?.eventType),
  },
  {
    accessorKey: 'target',
    header: ({ column }) => <SortableCell cellName="Target" column={column} />,
    cell: ({ row }) => {
      const { resourceType, resourceId, resourceName } = row.original;
      if (!resourceType) return null;

      const resourceConfig: Partial<Record<ActivityResourceType, { Icon: any; path: string }>> = {
        [ActivityResourceType.Deployment]: { Icon: Rocket, path: `/deployments/edit/${resourceId}` },
        [ActivityResourceType.Registry]: { Icon: Cable, path: `/registries/edit/${resourceId}` },
        [ActivityResourceType.Platform]: { Icon: Server, path: `/platforms/edit/${resourceId}` },
        [ActivityResourceType.Stack]: { Icon: Layers, path: `/stacks/edit/${resourceId}` },
      };

      const config = resourceConfig[resourceType];
      if (!config) return null;
      const { Icon, path } = config;

      return (
        <div className="flex flex-row items-center gap-2">
          <Icon width={13} height={13} className="text-foreground/80" />
          <Link to={path} className="table-link">
            {resourceName}
          </Link>
        </div>
      );
    },
    sortingFn: (rowA, rowB) => (rowA.original.platformName! < rowB.original.platformName! ? 1 : -1),
  },
  {
    accessorKey: 'status',
    header: ({ column }) => <SortableCell cellName="Status" column={column} />,
    cell: ({ row }) => <ActivityStatusCell status={row.original.status} />,
    sortingFn: (rowA, rowB) => (rowA.original.status! < rowB.original.status! ? 1 : -1),
  },
  {
    accessorKey: 'createdAt',
    header: ({ column }) => <SortableCell cellName="Start Time" column={column} />,
    cell: ({ row }) => {
      return <span>{new Date(row.original.createdAt).toLocaleString()}</span>;
    },
    sortingFn: (rowA, rowB) => (rowA.original.createdAt! < rowB.original.createdAt! ? 1 : -1),
  },
  {
    accessorKey: 'user',
    header: ({ column }) => <SortableCell cellName="User" column={column} />,
    cell: ({ row }) => (
      <ActorCell type={row.original.actorType} id={row.original.actorId ?? ''} name={row.original.actorName ?? ''} />
    ),
    sortingFn: (rowA, rowB) => (rowA.original.platformName! < rowB.original.platformName! ? 1 : -1),
  },
];
