import { useContainersGroup } from '../hooks/useContainersGroup';
import { ContainerProcessList, ProcessesSection } from './container-processes';
import { getSwarmTaskIdFromContainerName } from '@/lib/utils';
import { requestErrorStatus } from '@/lib/request-error';
import { Button } from '@/components/ui/button';

type RuntimeProcessTarget = { platformId: string } & (
  | { containerId: string; dockerNodeId?: string | null; taskId?: never }
  | { taskId: string; dockerNodeId: string; containerId?: never }
);

// Resolve the persisted UUID from the platform's live inventory. Docker IDs,
// task IDs and statistics IDs are not interchangeable with that identity.
export function RuntimeProcessList({ platformId, containerId, taskId, dockerNodeId }: RuntimeProcessTarget) {
  const { containersInfo, isLoading, error, refetch, isFetching } = useContainersGroup(platformId);
  const messageClass = 'border-t px-4 py-8 text-center text-sm text-muted-foreground';
  if (isLoading)
    return (
      <p className={messageClass} role="status">
        Loading container…
      </p>
    );
  if (requestErrorStatus(error) === 403 || (containersInfo && !containersInfo.capabilities?.canInspect)) {
    return <p className={messageClass}>Processes require Platform Read and Inspect permission.</p>;
  }
  if (error) {
    return (
      <div className="flex flex-col items-center gap-3 border-t px-4 py-8">
        <p role="alert" className="text-sm text-muted-foreground">
          Unable to load the container inventory.
        </p>
        <Button variant="outline" size="sm" disabled={isFetching} onClick={() => void refetch()}>
          Retry
        </Button>
      </div>
    );
  }
  const matches =
    containersInfo?.containers.filter(
      (container) =>
        container.platformId === platformId &&
        (dockerNodeId === undefined || container.dockerNodeId === dockerNodeId) &&
        (taskId
          ? container.isSwarmTask && getSwarmTaskIdFromContainerName(container.name) === taskId
          : container.containerId === containerId),
    ) ?? [];
  const container = matches.length === 1 ? matches[0] : undefined;
  if (!container) return <p className={messageClass}>The container is not available in the current inventory.</p>;
  if (container.projectionStaleSince != null) {
    return <p className={messageClass}>Processes are unavailable while the container inventory is stale.</p>;
  }
  return <ContainerProcessList key={container.id} resourceId={container.id} />;
}

export function RuntimeProcesses(props: RuntimeProcessTarget) {
  return (
    <ProcessesSection>
      <RuntimeProcessList {...props} />
    </ProcessesSection>
  );
}
