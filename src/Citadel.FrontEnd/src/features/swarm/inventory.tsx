import {
  ProblemDetails,
  SwarmConfigView,
  SwarmNetworkView,
  SwarmSecretView,
  SwarmServiceView,
  SwarmTaskView,
} from '@/api/generated/api.types';
import { AlertMessage } from '@/components/custom/alert-message';
import SortableCell from '@/components/custom/sortable-cell';
import { Badge } from '@/components/ui/badge';
import { DataTable } from '@/components/ui/data-table';
import { SwarmInventoryUpdate, useDockerDaemonGroup } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useRead } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { ColumnDef } from '@tanstack/react-table';
import { Boxes, FileCode2, KeyRound, ListTodo, Network } from 'lucide-react';
import { ComponentType, ReactNode, useCallback, useMemo } from 'react';
import { useNavigate, useParams } from 'react-router';

type QueryState<T> = { data?: { data?: { items: T[] } }; isLoading: boolean; error?: unknown };
type CachedResponse<T> = { data: T };

const useLiveItems = <T,>(
  platformId: string,
  resource: 'listSwarmServices' | 'listSwarmTasks' | 'listSwarmNetworks' | 'listSwarmSecrets' | 'listSwarmConfigs',
  query: QueryState<T>,
  select: (inventory: SwarmInventoryUpdate) => T[],
) => {
  const queryClient = useQueryClient();
  const queryArgs = useMemo(
    () => (resource === 'listSwarmTasks' ? { platformId, limit: 200 } : { platformId }),
    [platformId, resource],
  );
  const onSwarmInventoryUpdated = useCallback(
    (value: SwarmInventoryUpdate) => {
      if (value.platformId !== platformId) return;
      const queryKey = [resource, queryArgs] as const;
      const data = { items: select(value) };
      queryClient.setQueryData<CachedResponse<{ items: T[] }>>(queryKey, (previous) =>
        previous ? { ...previous, data } : { data },
      );
      void queryClient.cancelQueries({ queryKey, exact: true }, { revert: false });
    },
    [platformId, queryArgs, queryClient, resource, select],
  );
  useDockerDaemonGroup(platformId, { onSwarmInventoryUpdated });
  return query.data?.data?.items ?? [];
};

const nameCell = <T extends { id: string }>(
  value: string,
  resource: string,
  row: T,
  platformId: string,
  navigate: ReturnType<typeof useNavigate>,
  stale?: boolean,
) => (
  <button
    type="button"
    className="flex items-center gap-2 text-left font-medium hover:underline"
    onClick={() => navigate(`/platforms/${platformId}/swarm/${resource}/${row.id}`)}>
    <span>{value || row.id.slice(0, 12)}</span>
    {stale && <Badge variant="secondary">Stale</Badge>}
  </button>
);

const InventoryPage = <T,>({
  title,
  description,
  icon: Icon,
  query,
  items,
  columns,
}: {
  title: string;
  description: string;
  icon: ComponentType<{ className?: string }>;
  query: QueryState<T>;
  items: T[];
  columns: ColumnDef<T>[];
}) => {
  const problem = (query.error as { error?: ProblemDetails } | undefined)?.error;
  return (
    <div className="relative flex-col justify-between">
      <div className="mx-auto w-full max-w-[var(--layout-content-width)] px-4 py-4 sm:px-6">
        <div className="flex w-full flex-col gap-4 rounded-lg bg-background p-4">
          <div className="flex min-w-0 items-center gap-3">
            <div className="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
              <Icon className="h-4 w-4" />
            </div>
            <div>
              <h1 className="text-md font-bold text-foreground">{title}</h1>
              <p className="text-xs text-muted-foreground">{description}</p>
            </div>
          </div>
          {problem ? (
            <AlertMessage title={problem.title ?? `Unable to load ${title.toLowerCase()}`} type="error">
              {problem.detail ?? 'The Swarm inventory could not be loaded.'}
            </AlertMessage>
          ) : (
            <DataTable
              columns={columns}
              data={items}
              isLoading={query.isLoading}
              emptyState={{
                title: `No ${title.toLowerCase()} found.`,
                description: 'No matching resources were observed by the Swarm manager.',
              }}
            />
          )}
        </div>
      </div>
    </div>
  );
};

const serviceItems = (value: SwarmInventoryUpdate) => value.services.items;
const taskItems = (value: SwarmInventoryUpdate) => value.tasks.items.slice(0, 200);
const networkItems = (value: SwarmInventoryUpdate) => value.networks.items;
const secretItems = (value: SwarmInventoryUpdate) => value.secrets.items;
const configItems = (value: SwarmInventoryUpdate) => value.configs.items;
const observed = (value: string) => new Date(value).toLocaleString();
const list = (values: string[]): ReactNode => (values.length === 0 ? '-' : values.join(', '));

export const SwarmServicesPage = () => {
  const { platformId = '' } = useParams<{ platformId: string }>();
  const query = useRead('listSwarmServices', { platformId });
  const items = useLiveItems(platformId, 'listSwarmServices', query, serviceItems);
  const navigate = useNavigate();
  const columns = useMemo<ColumnDef<SwarmServiceView>[]>(
    () => [
      {
        accessorKey: 'name',
        header: ({ column }) => <SortableCell cellName="Service" column={column} />,
        cell: ({ row }) =>
          nameCell(row.original.name, 'services', row.original, platformId, navigate, row.original.isStale),
      },
      { accessorKey: 'mode', header: ({ column }) => <SortableCell cellName="Mode" column={column} /> },
      {
        id: 'replicas',
        header: 'Replicas',
        cell: ({ row }) => `${row.original.runningTaskCount}/${row.original.desiredTaskCount}`,
      },
      { accessorKey: 'updateState', header: ({ column }) => <SortableCell cellName="Update" column={column} /> },
      {
        accessorKey: 'image',
        header: 'Image',
        cell: ({ row }) => (
          <span className="block max-w-72 truncate" title={row.original.image}>
            {row.original.image}
          </span>
        ),
      },
      { accessorKey: 'observedAt', header: 'Last observed', cell: ({ row }) => observed(row.original.observedAt) },
    ],
    [navigate, platformId],
  );
  return (
    <InventoryPage
      title="Services"
      description="Swarm services and current replica state."
      icon={Boxes}
      query={query}
      items={items}
      columns={columns}
    />
  );
};

export const SwarmTasksPage = () => {
  const { platformId = '' } = useParams<{ platformId: string }>();
  const query = useRead('listSwarmTasks', { platformId, limit: 200 });
  const items = useLiveItems(platformId, 'listSwarmTasks', query, taskItems);
  const navigate = useNavigate();
  const columns = useMemo<ColumnDef<SwarmTaskView>[]>(
    () => [
      {
        accessorKey: 'name',
        header: ({ column }) => <SortableCell cellName="Task" column={column} />,
        cell: ({ row }) =>
          nameCell(
            row.original.name || `Task ${row.original.slot ?? ''}`,
            'tasks',
            row.original,
            platformId,
            navigate,
            row.original.isStale,
          ),
      },
      {
        accessorKey: 'serviceName',
        header: ({ column }) => <SortableCell cellName="Service" column={column} />,
        cell: ({ row }) => row.original.serviceName || row.original.serviceId.slice(0, 12),
      },
      {
        accessorKey: 'nodeHostname',
        header: ({ column }) => <SortableCell cellName="Node" column={column} />,
        cell: ({ row }) => row.original.nodeHostname || row.original.nodeId.slice(0, 12),
      },
      { accessorKey: 'state', header: ({ column }) => <SortableCell cellName="State" column={column} /> },
      { accessorKey: 'desiredState', header: ({ column }) => <SortableCell cellName="Desired" column={column} /> },
      {
        accessorKey: 'image',
        header: 'Image',
        cell: ({ row }) => (
          <span className="block max-w-64 truncate" title={row.original.image}>
            {row.original.image}
          </span>
        ),
      },
      {
        accessorKey: 'statusTimestamp',
        header: 'Updated',
        cell: ({ row }) => (row.original.statusTimestamp ? observed(row.original.statusTimestamp) : '-'),
      },
    ],
    [navigate, platformId],
  );
  return (
    <InventoryPage
      title="Tasks"
      description="Current scheduler-owned tasks. Tasks are read-only."
      icon={ListTodo}
      query={query}
      items={items}
      columns={columns}
    />
  );
};

export const SwarmNetworksPage = () => {
  const { platformId = '' } = useParams<{ platformId: string }>();
  const query = useRead('listSwarmNetworks', { platformId });
  const items = useLiveItems(platformId, 'listSwarmNetworks', query, networkItems);
  const navigate = useNavigate();
  const columns = useMemo<ColumnDef<SwarmNetworkView>[]>(
    () => [
      {
        accessorKey: 'name',
        header: ({ column }) => <SortableCell cellName="Network" column={column} />,
        cell: ({ row }) =>
          nameCell(row.original.name, 'networks', row.original, platformId, navigate, row.original.isStale),
      },
      { accessorKey: 'driver', header: ({ column }) => <SortableCell cellName="Driver" column={column} /> },
      { accessorKey: 'scope', header: ({ column }) => <SortableCell cellName="Scope" column={column} /> },
      { id: 'services', header: 'Services', cell: ({ row }) => list(row.original.serviceNames) },
      {
        id: 'options',
        header: 'Options',
        cell: ({ row }) =>
          list(
            [
              row.original.isIngress ? 'Ingress' : '',
              row.original.isAttachable ? 'Attachable' : '',
              row.original.isEncrypted ? 'Encrypted' : '',
            ].filter(Boolean),
          ),
      },
      { accessorKey: 'observedAt', header: 'Last observed', cell: ({ row }) => observed(row.original.observedAt) },
    ],
    [navigate, platformId],
  );
  return (
    <InventoryPage
      title="Networks"
      description="Cluster-scoped Swarm networks and service references."
      icon={Network}
      query={query}
      items={items}
      columns={columns}
    />
  );
};

export const SwarmSecretsPage = () => {
  const { platformId = '' } = useParams<{ platformId: string }>();
  const query = useRead('listSwarmSecrets', { platformId });
  const items = useLiveItems(platformId, 'listSwarmSecrets', query, secretItems);
  const navigate = useNavigate();
  const columns = useMemo<ColumnDef<SwarmSecretView>[]>(
    () => [
      {
        accessorKey: 'name',
        header: ({ column }) => <SortableCell cellName="Secret" column={column} />,
        cell: ({ row }) =>
          nameCell(row.original.name, 'secrets', row.original, platformId, navigate, row.original.isStale),
      },
      { accessorKey: 'driver', header: 'Driver', cell: ({ row }) => row.original.driver ?? 'Swarm' },
      { id: 'services', header: 'Used by', cell: ({ row }) => list(row.original.serviceNames) },
      {
        accessorKey: 'createdAt',
        header: 'Created',
        cell: ({ row }) => (row.original.createdAt ? observed(row.original.createdAt) : '-'),
      },
      { accessorKey: 'observedAt', header: 'Last observed', cell: ({ row }) => observed(row.original.observedAt) },
    ],
    [navigate, platformId],
  );
  return (
    <InventoryPage
      title="Secrets"
      description="Secret metadata only. Citadel never reads or returns secret data."
      icon={KeyRound}
      query={query}
      items={items}
      columns={columns}
    />
  );
};

export const SwarmConfigsPage = () => {
  const { platformId = '' } = useParams<{ platformId: string }>();
  const query = useRead('listSwarmConfigs', { platformId });
  const items = useLiveItems(platformId, 'listSwarmConfigs', query, configItems);
  const navigate = useNavigate();
  const columns = useMemo<ColumnDef<SwarmConfigView>[]>(
    () => [
      {
        accessorKey: 'name',
        header: ({ column }) => <SortableCell cellName="Config" column={column} />,
        cell: ({ row }) =>
          nameCell(row.original.name, 'configs', row.original, platformId, navigate, row.original.isStale),
      },
      {
        accessorKey: 'templatingDriver',
        header: 'Templating',
        cell: ({ row }) => row.original.templatingDriver ?? '-',
      },
      { id: 'services', header: 'Used by', cell: ({ row }) => list(row.original.serviceNames) },
      {
        accessorKey: 'createdAt',
        header: 'Created',
        cell: ({ row }) => (row.original.createdAt ? observed(row.original.createdAt) : '-'),
      },
      { accessorKey: 'observedAt', header: 'Last observed', cell: ({ row }) => observed(row.original.observedAt) },
    ],
    [navigate, platformId],
  );
  return (
    <InventoryPage
      title="Configs"
      description="Swarm config metadata and service references."
      icon={FileCode2}
      query={query}
      items={items}
      columns={columns}
    />
  );
};
