import { SwarmNodeAgentCoverage } from '@/api/generated/api.types';
import { ActionWithDialog } from '@/components/custom/action-with-dialog';
import { AlertMessage } from '@/components/custom/alert-message';
import { Button } from '@/components/ui/button';
import Loader from '@/components/ui/loader';
import { useDockerDaemonGroup } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useTaskSheet } from '@/lib/atoms';
import { useRead } from '@/lib/hooks';
import { cn } from '@/lib/utils';
import { useQueryClient } from '@tanstack/react-query';
import { RefreshCw, ShieldCheck, Trash2, TriangleAlert } from 'lucide-react';
import { useCallback, useMemo } from 'react';

type NodeAgentAction = 'install' | 'repair' | 'upgrade' | 'remove';

export const SwarmNodeAgentCoveragePanel = ({
  platformId,
  platformName,
}: {
  platformId: string;
  platformName: string;
}) => {
  const args = useMemo(() => ({ id: platformId }), [platformId]);
  const query = useRead('getSwarmNodeAgentCoverage', args);
  const queryClient = useQueryClient();
  const taskSheet = useTaskSheet('Platform');
  const coverage = query.data?.data;
  const operationRunning = coverage?.operation?.state === 'Running';
  const requiresSatellites = coverage?.nodes.some((node) => node.dataSource === 'Satellite' && node.eligible);
  const installed = coverage?.isInstalled === true;

  const refreshCoverage = useCallback(() => {
    void queryClient.invalidateQueries({ queryKey: ['getSwarmNodeAgentCoverage', args] });
  }, [args, queryClient]);
  useDockerDaemonGroup(platformId, {
    onSwarmInventoryUpdated: refreshCoverage,
    onSwarmNodeAgentCoverageChanged: refreshCoverage,
  });

  const run = (action: NodeAgentAction) =>
    taskSheet.open({ kind: 'swarmNodeAgents', payload: { platformId, name: platformName, action } });

  if (query.isLoading && !coverage) return <Loader />;
  if (query.error) {
    return (
      <AlertMessage title="Node data plane unavailable" type="error">
        The node-agent coverage could not be loaded.
      </AlertMessage>
    );
  }
  if (!coverage) return null;

  const disabled = !coverage.canManageNodeAgents || operationRunning;

  return (
    <section className="mb-1 overflow-hidden rounded-md border bg-background" aria-label="Cluster node coverage">
      <div className="flex flex-wrap items-center justify-between gap-3 border-b px-4 py-3">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <ShieldCheck className="size-4 text-muted-foreground" />
            <h2 className="text-sm font-semibold">Cluster node coverage</h2>
            <CoverageState value={coverage.state} />
          </div>
          <p className="mt-1 text-xs text-muted-foreground">
            {coverage.coveredNodes}/{coverage.eligibleNodes} eligible nodes have a usable local data source.
          </p>
          {coverage.agentImageReference && (
            <p
              className="mt-1 max-w-2xl truncate text-xs text-muted-foreground"
              title={coverage.agentImageDigest ?? undefined}>
              Agent image: {coverage.agentImageReference}
            </p>
          )}
        </div>
        <div className="flex flex-wrap items-center justify-end gap-2">
          {!installed && requiresSatellites ? (
            <Button size="sm" disabled={disabled} onClick={() => run('install')}>
              Install node agents
            </Button>
          ) : installed ? (
            <>
              <Button size="sm" variant="outline" disabled={disabled} onClick={() => run('repair')}>
                <RefreshCw className="size-3.5" /> Repair
              </Button>
              <Button size="sm" variant="outline" disabled={disabled} onClick={() => run('upgrade')}>
                Upgrade
              </Button>
              <ActionWithDialog
                name={platformName}
                title="Remove node data plane"
                icon={<Trash2 className="size-3.5" />}
                variant="destructive"
                disabled={disabled}
                onClick={() => run('remove')}
                description="This removes Citadel's node-agent Service and credentials. Docker workloads and persistent Agent identity volumes are preserved."
              />
            </>
          ) : null}
        </div>
      </div>

      {coverage.operation?.error && (
        <div className="px-4 pt-2">
          <AlertMessage title="Node data-plane operation failed" type="error">
            {coverage.operation.error}
          </AlertMessage>
        </div>
      )}

      {coverage.reasons.includes('NodeAgentServiceDrifted') && (
        <div className="px-4 pt-2">
          <AlertMessage title="Node-agent Service drift detected" type="warning">
            Docker no longer matches Citadel’s installed node-agent Service. Run Repair to restore coverage.
          </AlertMessage>
        </div>
      )}

      {coverage.reasons.includes('SwarmInventoryUnavailable') && (
        <div className="px-4 pt-2">
          <AlertMessage title="Swarm inventory unavailable" type="warning">
            Citadel cannot calculate node coverage until the current Swarm node inventory is available.
          </AlertMessage>
        </div>
      )}

      <div className="flex flex-wrap gap-x-4 gap-y-1 border-b px-4 py-2 text-xs text-muted-foreground">
        <span>{coverage.totalNodes} total</span>
        <span>{coverage.connectedNodes} connected</span>
        <span>{coverage.missingNodes} missing</span>
        <span>{coverage.offlineNodes} offline</span>
        <span>{coverage.incompatibleNodes} incompatible</span>
        <span>{coverage.unsupportedNodes} unsupported</span>
        <span>{coverage.unschedulableNodes} unschedulable</span>
        <span>{coverage.staleNodes} stale</span>
        {coverage.lastMembershipReconciliationAtUtc && (
          <span title={new Date(coverage.lastMembershipReconciliationAtUtc).toLocaleString()}>
            Membership last reconciled {new Date(coverage.lastMembershipReconciliationAtUtc).toLocaleString()}
          </span>
        )}
      </div>

      {requiresSatellites && (
        <div className="flex items-start gap-2 border-b bg-amber-500/5 px-4 py-2 text-xs text-muted-foreground">
          <TriangleAlert className="mt-0.5 size-3.5 shrink-0 text-amber-600" />
          <span>Each satellite mounts its local Docker socket and has daemon-level authority on that node.</span>
        </div>
      )}

      <div className="divide-y">
        {coverage.nodes.map((node) => (
          <div
            key={node.dockerNodeId}
            className="flex flex-wrap items-center justify-between gap-3 px-4 py-2.5 text-xs">
            <div className="flex min-w-0 items-center gap-2">
              <span
                className={cn(
                  'size-2 shrink-0 rounded-full',
                  node.dockerReachable && !node.projectionStale
                    ? 'bg-emerald-500'
                    : node.eligible
                      ? 'bg-amber-500'
                      : 'bg-muted-foreground/40',
                )}
                aria-hidden="true"
              />
              <span className="truncate font-medium">{node.hostname || node.dockerNodeId.slice(0, 12)}</span>
              <span className="text-muted-foreground">{node.role}</span>
            </div>
            <div className="flex flex-wrap items-center justify-end gap-x-4 gap-y-1 text-muted-foreground">
              <span>{node.architecture}</span>
              <span>{node.dataSource === 'ManagerConnector' ? 'Manager connector' : node.agentConnectionState}</span>
              {node.reasons.length > 0 && <span className="text-amber-600">{node.reasons.join(', ')}</span>}
            </div>
          </div>
        ))}
      </div>
    </section>
  );
};

const CoverageState = ({ value }: { value: SwarmNodeAgentCoverage['state'] }) => (
  <span
    className={cn(
      'rounded-sm px-1.5 py-0.5 text-[11px] font-medium',
      value === 'Complete'
        ? 'bg-emerald-500/10 text-emerald-700 dark:text-emerald-400'
        : value === 'Failed'
          ? 'bg-destructive/10 text-destructive'
          : value === 'NotInstalled'
            ? 'bg-muted text-muted-foreground'
            : 'bg-amber-500/10 text-amber-700 dark:text-amber-400',
    )}>
    {value.replace(/([a-z])([A-Z])/g, '$1 $2')}
  </span>
);
