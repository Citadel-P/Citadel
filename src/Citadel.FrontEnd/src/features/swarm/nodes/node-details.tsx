import { ProblemDetails, SwarmNodeView } from '@/api/generated/api.types';
import { SwarmInventoryUpdate } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { AlertMessage } from '@/components/custom/alert-message';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import Loader from '@/components/ui/loader';
import { useDockerDaemonGroup } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useRead } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { ArrowLeft, Network } from 'lucide-react';
import { useCallback } from 'react';
import { useNavigate, useParams } from 'react-router';

type CachedResponse<T> = { data: T };

export const SwarmNodeDetailsPage = () => {
  const { platformId = '', resourceId: nodeId = '' } = useParams<{ platformId: string; resourceId: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const query = useRead('getSwarmNode', { platformId, nodeId });

  const onSwarmInventoryUpdated = useCallback(
    (updated: SwarmInventoryUpdate) => {
      if (updated.platformId !== platformId) return;
      const current = updated.nodes.items.find((item) => item.id === nodeId);
      if (current) {
        const queryKey = ['getSwarmNode', { platformId, nodeId }] as const;
        queryClient.setQueryData<CachedResponse<SwarmNodeView>>(queryKey, (previous) =>
          previous ? { ...previous, data: current } : { data: current },
        );
        void queryClient.cancelQueries({ queryKey, exact: true }, { revert: false });
      } else {
        navigate(`/platforms/${platformId}/swarm/nodes`, { replace: true });
      }
    },
    [navigate, nodeId, platformId, queryClient],
  );
  useDockerDaemonGroup(platformId, { onSwarmInventoryUpdated });
  const node = query.data?.data;

  const problem = (query.error as unknown as { error?: ProblemDetails } | undefined)?.error;
  return (
    <div className="flex-col justify-between relative">
      <div className="mx-auto w-full max-w-[var(--layout-content-width)] px-4 py-4 sm:px-6">
        <div className="flex w-full flex-col gap-4 rounded-lg bg-background p-4">
          <Button
            type="button"
            variant="ghost"
            className="w-fit px-2"
            onClick={() => navigate(`/platforms/${platformId}/swarm/nodes`)}>
            <ArrowLeft className="h-4 w-4" /> Nodes
          </Button>
          {query.isLoading && !node ? (
            <Loader />
          ) : problem ? (
            <AlertMessage title={problem.title ?? 'Unable to load node'} type="error">
              {problem.detail ?? 'The Swarm node could not be loaded.'}
            </AlertMessage>
          ) : node ? (
            <>
              <div className="flex min-w-0 items-center gap-3">
                <div className="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                  <Network className="h-4 w-4" />
                </div>
                <div className="min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <h1 className="truncate text-md font-bold text-foreground">{node.hostname || node.id}</h1>
                    {node.isLeader && <Badge variant="outline">Leader</Badge>}
                    {node.isStale && <Badge variant="secondary">Stale</Badge>}
                  </div>
                  <p className="truncate text-xs text-muted-foreground">{node.id}</p>
                </div>
              </div>
              {node.isStale && (
                <AlertMessage title="Last-known node state" type="warning">
                  Citadel could not complete the latest Swarm inventory read. This information may be out of date.
                </AlertMessage>
              )}
              <section className="grid gap-x-8 gap-y-4 rounded-md border border-border p-4 sm:grid-cols-2 lg:grid-cols-3">
                <Detail label="Status" value={node.status} />
                <Detail label="Availability" value={node.availability} />
                <Detail label="Role" value={node.role} />
                <Detail label="Reachability" value={node.reachability} />
                <Detail label="Address" value={node.address || '-'} />
                <Detail label="Tasks" value={`${node.runningTaskCount}/${node.desiredTaskCount} running`} />
                <Detail label="Engine" value={node.engineVersion || '-'} />
                <Detail label="Operating system" value={node.operatingSystem || '-'} />
                <Detail label="Architecture" value={node.architecture || '-'} />
                <Detail label="Last observed" value={new Date(node.observedAt).toLocaleString()} />
              </section>
              <section className="rounded-md border border-border p-4">
                <h2 className="mb-3 text-sm font-semibold">Labels</h2>
                {Object.keys(node.labels).length === 0 ? (
                  <p className="text-sm text-muted-foreground">No node labels.</p>
                ) : (
                  <div className="flex flex-wrap gap-2">
                    {Object.entries(node.labels).map(([key, value]) => (
                      <Badge key={key} variant="secondary">{`${key}=${value}`}</Badge>
                    ))}
                  </div>
                )}
              </section>
            </>
          ) : null}
        </div>
      </div>
    </div>
  );
};

const Detail = ({ label, value }: { label: string; value: string }) => (
  <div className="min-w-0">
    <div className="text-xs text-muted-foreground">{label}</div>
    <div className="truncate text-sm text-foreground" title={value}>
      {value}
    </div>
  </div>
);

export default SwarmNodeDetailsPage;
