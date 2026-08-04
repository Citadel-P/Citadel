import { ProblemDetails } from '@/api/generated/api.types';
import { AlertMessage } from '@/components/custom/alert-message';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import Loader from '@/components/ui/loader';
import { SwarmInventoryUpdate, useDockerDaemonGroup } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useRead } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { ArrowLeft, Boxes, FileCode2, KeyRound, ListTodo, Network } from 'lucide-react';
import { ComponentType, ReactNode, useCallback } from 'react';
import { Link, useNavigate, useParams } from 'react-router';

type Resource = { id: string; name: string; labels?: Record<string, string>; isStale: boolean; observedAt: string };
type QueryState<T> = { data?: { data?: T }; isLoading: boolean; error?: unknown };
type CachedResponse<T> = { data: T };

const useLiveResource = <T extends Resource>(
  platformId: string,
  resourceId: string,
  resource: 'getSwarmService' | 'getSwarmTask' | 'getSwarmNetwork' | 'getSwarmSecret' | 'getSwarmConfig',
  listPath: string,
  query: QueryState<T>,
  select: (inventory: SwarmInventoryUpdate) => T[],
) => {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const onSwarmInventoryUpdated = useCallback(
    (inventory: SwarmInventoryUpdate) => {
      if (inventory.platformId !== platformId) return;
      const current = select(inventory).find((item) => item.id === resourceId);
      if (current) {
        const queryKey = [resource, { platformId, resourceId }] as const;
        queryClient.setQueryData<CachedResponse<T>>(queryKey, (previous) =>
          previous ? { ...previous, data: current } : { data: current },
        );
        void queryClient.cancelQueries({ queryKey, exact: true }, { revert: false });
      }
      else navigate(listPath, { replace: true });
    },
    [listPath, navigate, platformId, queryClient, resource, resourceId, select],
  );
  useDockerDaemonGroup(platformId, { onSwarmInventoryUpdated });
  return query.data?.data;
};

const DetailPage = <T extends Resource>({
  title,
  listLabel,
  listPath,
  icon: Icon,
  query,
  resource,
  details,
  note,
  serviceNames,
}: {
  title: string;
  listLabel: string;
  listPath: string;
  icon: ComponentType<{ className?: string }>;
  query: QueryState<T>;
  resource?: T;
  details: { label: string; value: ReactNode; title?: string }[];
  note?: string;
  serviceNames?: string[];
}) => {
  const navigate = useNavigate();
  const problem = (query.error as { error?: ProblemDetails } | undefined)?.error;
  return (
    <div className="relative flex-col justify-between">
      <div className="mx-auto w-full max-w-[var(--layout-content-width)] px-4 py-4 sm:px-6">
        <div className="flex w-full flex-col gap-4 rounded-lg bg-background p-4">
          <Button type="button" variant="ghost" className="w-fit px-2" onClick={() => navigate(listPath)}>
            <ArrowLeft className="h-4 w-4" /> {listLabel}
          </Button>
          {query.isLoading && !resource ? (
            <Loader />
          ) : problem ? (
            <AlertMessage title={problem.title ?? `Unable to load ${title.toLowerCase()}`} type="error">
              {problem.detail ?? 'The Swarm resource could not be loaded.'}
            </AlertMessage>
          ) : resource ? (
            <>
              <div className="flex min-w-0 items-center gap-3">
                <div className="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                  <Icon className="h-4 w-4" />
                </div>
                <div className="min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <h1 className="truncate text-md font-bold text-foreground">{resource.name || resource.id}</h1>
                    {resource.isStale && <Badge variant="secondary">Stale</Badge>}
                  </div>
                  <p className="truncate text-xs text-muted-foreground">{resource.id}</p>
                </div>
              </div>
              {resource.isStale && (
                <AlertMessage title="Last-known state" type="warning">
                  Citadel could not complete the latest Swarm inventory read. This information may be out of date.
                </AlertMessage>
              )}
              {note && (
                <AlertMessage title={title} type="info">
                  {note}
                </AlertMessage>
              )}
              <section className="grid gap-x-8 gap-y-4 rounded-md border border-border p-4 sm:grid-cols-2 lg:grid-cols-3">
                {details.map((detail) => (
                  <Detail key={detail.label} label={detail.label} value={detail.value} title={detail.title} />
                ))}
                <Detail label="Last observed" value={new Date(resource.observedAt).toLocaleString()} />
              </section>
              {serviceNames && (
                <section className="rounded-md border border-border p-4">
                  <h2 className="mb-3 text-sm font-semibold">Used by services</h2>
                  <div className="flex flex-wrap gap-2">
                    {serviceNames.length ? (
                      serviceNames.map((name) => (
                        <Badge key={name} variant="secondary">
                          {name}
                        </Badge>
                      ))
                    ) : (
                      <span className="text-sm text-muted-foreground">Not referenced by a service.</span>
                    )}
                  </div>
                </section>
              )}
              <section className="rounded-md border border-border p-4">
                <h2 className="mb-3 text-sm font-semibold">Labels</h2>
                <div className="flex flex-wrap gap-2">
                  {Object.entries(resource.labels ?? {}).length ? (
                    Object.entries(resource.labels ?? {}).map(([key, value]) => (
                      <Badge key={key} variant="secondary">{`${key}=${value}`}</Badge>
                    ))
                  ) : (
                    <span className="text-sm text-muted-foreground">No labels.</span>
                  )}
                </div>
              </section>
            </>
          ) : null}
        </div>
      </div>
    </div>
  );
};

const Detail = ({ label, value, title }: { label: string; value: ReactNode; title?: string }) => (
  <div className="min-w-0">
    <div className="text-xs text-muted-foreground">{label}</div>
    <div className="truncate text-sm text-foreground" title={title}>
      {value}
    </div>
  </div>
);
const display = (values: string[]) => (values.length ? values.join(', ') : '-');
const serviceItems = (value: SwarmInventoryUpdate) => value.services.items;
const taskItems = (value: SwarmInventoryUpdate) => value.tasks.items;
const networkItems = (value: SwarmInventoryUpdate) => value.networks.items;
const secretItems = (value: SwarmInventoryUpdate) => value.secrets.items;
const configItems = (value: SwarmInventoryUpdate) => value.configs.items;

export const SwarmServiceDetailsPage = () => {
  const { platformId = '', resourceId = '' } = useParams<{ platformId: string; resourceId: string }>();
  const path = `/platforms/${platformId}/swarm/services`;
  const query = useRead('getSwarmService', { platformId, resourceId });
  const value = useLiveResource(platformId, resourceId, 'getSwarmService', path, query, serviceItems);
  return (
    <DetailPage
      title="Service"
      listLabel="Services"
      listPath={path}
      icon={Boxes}
      query={query}
      resource={value}
      details={
        value
          ? [
              { label: 'Mode', value: value.mode },
              { label: 'Replicas', value: `${value.runningTaskCount}/${value.desiredTaskCount}` },
              { label: 'Update', value: value.updateState },
              { label: 'Image', value: value.image || '-', title: value.image },
              { label: 'Ports', value: display(value.ports), title: display(value.ports) },
              { label: 'Created', value: value.createdAt ? new Date(value.createdAt).toLocaleString() : '-' },
            ]
          : []
      }
    />
  );
};

export const SwarmTaskDetailsPage = () => {
  const { platformId = '', resourceId = '' } = useParams<{ platformId: string; resourceId: string }>();
  const path = `/platforms/${platformId}/swarm/tasks`;
  const query = useRead('getSwarmTask', { platformId, resourceId });
  const value = useLiveResource(platformId, resourceId, 'getSwarmTask', path, query, taskItems);
  return (
    <DetailPage
      title="Task"
      listLabel="Tasks"
      listPath={path}
      icon={ListTodo}
      query={query}
      resource={value}
      note="Tasks are immutable and scheduler-owned. Runtime changes are made through the owning service or deployment."
      details={
        value
          ? [
              { label: 'State', value: value.state },
              { label: 'Desired state', value: value.desiredState },
              { label: 'Service', value: value.serviceName || value.serviceId, title: value.serviceId },
              {
                label: 'Node',
                value: (
                  <Link className="hover:underline" to={`/platforms/${platformId}/swarm/nodes/${value.nodeId}`}>
                    {value.nodeHostname || value.nodeId}
                  </Link>
                ),
                title: value.nodeId,
              },
              { label: 'Slot', value: value.slot ?? '-' },
              { label: 'Image', value: value.image || '-', title: value.image },
              { label: 'Message', value: value.statusMessage ?? '-', title: value.statusMessage ?? undefined },
              { label: 'Error', value: value.error ?? '-', title: value.error ?? undefined },
            ]
          : []
      }
    />
  );
};

export const SwarmNetworkDetailsPage = () => {
  const { platformId = '', resourceId = '' } = useParams<{ platformId: string; resourceId: string }>();
  const path = `/platforms/${platformId}/swarm/networks`;
  const query = useRead('getSwarmNetwork', { platformId, resourceId });
  const value = useLiveResource(platformId, resourceId, 'getSwarmNetwork', path, query, networkItems);
  return (
    <DetailPage
      title="Network"
      listLabel="Networks"
      listPath={path}
      icon={Network}
      query={query}
      resource={value}
      serviceNames={value?.serviceNames}
      details={
        value
          ? [
              { label: 'Driver', value: value.driver },
              { label: 'Scope', value: value.scope },
              { label: 'Subnets', value: display(value.subnets), title: display(value.subnets) },
              { label: 'Attachable', value: value.isAttachable ? 'Yes' : 'No' },
              { label: 'Internal', value: value.isInternal ? 'Yes' : 'No' },
              { label: 'Ingress', value: value.isIngress ? 'Yes' : 'No' },
              { label: 'Encrypted', value: value.isEncrypted ? 'Yes' : 'No' },
              { label: 'IPv6', value: value.enableIPv6 ? 'Enabled' : 'Disabled' },
            ]
          : []
      }
    />
  );
};

export const SwarmSecretDetailsPage = () => {
  const { platformId = '', resourceId = '' } = useParams<{ platformId: string; resourceId: string }>();
  const path = `/platforms/${platformId}/swarm/secrets`;
  const query = useRead('getSwarmSecret', { platformId, resourceId });
  const value = useLiveResource(platformId, resourceId, 'getSwarmSecret', path, query, secretItems);
  return (
    <DetailPage
      title="Secret"
      listLabel="Secrets"
      listPath={path}
      icon={KeyRound}
      query={query}
      resource={value}
      serviceNames={value?.serviceNames}
      note="Only metadata and service references are available. Docker never returns the secret value."
      details={
        value
          ? [
              { label: 'Driver', value: value.driver ?? 'Swarm' },
              { label: 'Created', value: value.createdAt ? new Date(value.createdAt).toLocaleString() : '-' },
              { label: 'Updated', value: value.updatedAt ? new Date(value.updatedAt).toLocaleString() : '-' },
            ]
          : []
      }
    />
  );
};

export const SwarmConfigDetailsPage = () => {
  const { platformId = '', resourceId = '' } = useParams<{ platformId: string; resourceId: string }>();
  const path = `/platforms/${platformId}/swarm/configs`;
  const query = useRead('getSwarmConfig', { platformId, resourceId });
  const value = useLiveResource(platformId, resourceId, 'getSwarmConfig', path, query, configItems);
  return (
    <DetailPage
      title="Config"
      listLabel="Configs"
      listPath={path}
      icon={FileCode2}
      query={query}
      resource={value}
      serviceNames={value?.serviceNames}
      details={
        value
          ? [
              { label: 'Templating driver', value: value.templatingDriver ?? '-' },
              { label: 'Created', value: value.createdAt ? new Date(value.createdAt).toLocaleString() : '-' },
              { label: 'Updated', value: value.updatedAt ? new Date(value.updatedAt).toLocaleString() : '-' },
            ]
          : []
      }
    />
  );
};
