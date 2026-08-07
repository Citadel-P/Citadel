import {
  PlatformCapabilities,
  PlatformStatus,
  ProblemDetails,
  StackReleaseStatus,
  SwarmServiceOwnership,
  SwarmServiceView,
} from '@/api/generated/api.types';
import { AlertMessage } from '@/components/custom/alert-message';
import { LogViewer } from '@/components/custom/common';
import { Badge } from '@/components/ui/badge';
import { AppContext } from '@/lib/context/app-context';
import { useRead } from '@/lib/hooks';
import { ReactNode, useContext } from 'react';

export const NoResourceActions = () => null;

export const StaleBadge = ({ resource }: { resource: { isStale: boolean } }) =>
  resource.isStale ? <Badge variant="secondary">Stale</Badge> : null;

export const StaleWarning = ({ resource }: { resource: { isStale: boolean } }) =>
  resource.isStale ? (
    <AlertMessage title="Last-known state" type="warning">
      Citadel could not complete the latest Swarm inventory read. This information may be out of date.
    </AlertMessage>
  ) : null;

export const Details = ({ items }: { items: { label: string; value: ReactNode; title?: string }[] }) => (
  <section className="grid gap-x-8 gap-y-4 rounded-md border border-border p-4 sm:grid-cols-2 lg:grid-cols-3">
    {items.map((item) => (
      <Detail key={item.label} {...item} />
    ))}
  </section>
);

export const Detail = ({ label, value, title }: { label: string; value: ReactNode; title?: string }) => (
  <div className="min-w-0">
    <div className="text-xs text-muted-foreground">{label}</div>
    <div className="truncate text-sm text-foreground" title={title}>
      {value}
    </div>
  </div>
);

export const SwarmLogs = ({
  platformId,
  resourceId,
  resource,
  capabilities,
}: {
  platformId: string;
  resourceId: string;
  resource: 'service' | 'task';
  capabilities?: PlatformCapabilities;
}) => {
  const appContext = useContext(AppContext);
  const currentPlatform = appContext?.currentPlatform;
  const canViewLogs = capabilities?.canViewLogs === true;
  const isOnline =
    !appContext || (currentPlatform?.id === platformId && currentPlatform.status === PlatformStatus.Online);
  const serviceLogs = useRead(
    'getSwarmServiceLogs',
    { platformId, resourceId, query: { tail: 100 } },
    { enabled: resource === 'service' && canViewLogs && isOnline },
  );
  const taskLogs = useRead(
    'getSwarmTaskLogs',
    { platformId, resourceId, query: { tail: 100 } },
    { enabled: resource === 'task' && canViewLogs && isOnline },
  );

  if (!canViewLogs || !isOnline) return null;

  const query = resource === 'service' ? serviceLogs : taskLogs;
  const problem = (query.error as { error?: ProblemDetails } | undefined)?.error;
  if (problem) {
    return (
      <AlertMessage title={problem.title ?? 'Unable to load logs'} type="error">
        {problem.detail ?? 'Docker could not return logs for this resource.'}
      </AlertMessage>
    );
  }

  return (
    <>
      <LogViewer logs={query.data?.data.lines ?? []} autoScroll={false} allowWrap timeStamps />
      {query.data?.data.truncated && (
        <p className="mt-2 text-xs text-muted-foreground">The response reached the 1 MiB safety limit.</p>
      )}
    </>
  );
};

export const displayList = (values: string[]) => (values.length ? values.join(', ') : '-');

export const swarmOwnershipLabel = (value: SwarmServiceOwnership) => {
  if (value === SwarmServiceOwnership.CitadelDeployment) return 'Citadel Deployment';
  if (value === SwarmServiceOwnership.CitadelStack) return 'Citadel Stack';
  if (value === SwarmServiceOwnership.DockerStackExternal) return 'External Docker Stack';
  if (value === SwarmServiceOwnership.CitadelService) return 'Citadel Service';
  if (value === SwarmServiceOwnership.OwnershipConflict) return 'Ownership Conflict';
  return 'Unmanaged';
};

export const getServiceAvailability = (
  service: Pick<SwarmServiceView, 'desiredTaskCount' | 'isStale' | 'runningTaskCount' | 'updateState'>,
) => {
  const running = Number(service.runningTaskCount);
  const desired = Number(service.desiredTaskCount);
  const tasks = `${running}/${desired} tasks running`;

  if (service.isStale) return { status: StackReleaseStatus.Unknown, tooltip: `Last-known state: ${tasks}` };
  if (desired === 0) return { status: StackReleaseStatus.Stopped, tooltip: 'No active tasks requested' };
  if (service.updateState.toLowerCase().includes('paused')) {
    return { status: StackReleaseStatus.Degraded, tooltip: `Service update paused; ${tasks}` };
  }
  if (running >= desired) return { status: StackReleaseStatus.Healthy, tooltip: tasks };
  if (running > 0) return { status: StackReleaseStatus.Degraded, tooltip: tasks };
  return { status: StackReleaseStatus.Failed, tooltip: tasks };
};
