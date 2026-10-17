import { useContext } from 'react';
import { Activity } from 'lucide-react';
import {
  ContainerHealthStatus,
  PlatformStatus,
  type SwarmServiceView,
  type SwarmTaskView,
} from '@/api/generated/api.types';
import { HealthCard } from '@/components/custom/health-card';
import { HealthLabel, healthLabels, healthTone, useHealthClock } from '@/components/custom/health-status';
import { groupHealth } from '@/features/docker-resources/containers/group-health';
import { RealtimeContext } from '@/lib/context/realtime-context';
import { type HealthResource, memberHealth } from './health';
import { WorkloadHealthMembers, WorkloadMemberHealthCard, UnavailableHealthCard } from './card';

type Service = Pick<SwarmServiceView, 'id' | 'name' | 'desiredTaskCount' | 'isStale'> & { tasks: SwarmTaskView[] };

// Core already selects current tasks by slot/node and resolves their exact container
// identity. Do not reconstruct membership from Docker names or include old attempts.
export function serviceHealthMembers(resource: HealthResource, service: Service, platformId: string) {
  return (
    resource.probeHealth?.members.filter(
      (member) =>
        member.platformId === platformId &&
        member.service === service.id &&
        service.tasks.some(
          (task) =>
            task.id === member.taskId &&
            task.nodeId === member.nodeId &&
            task.serviceId === service.id &&
            task.desiredState.toLowerCase() === 'running',
        ),
    ) ?? []
  );
}

export function WorkloadServiceHealthCard({
  resource,
  service,
  platformId,
}: {
  resource: HealthResource;
  service: Service;
  platformId: string;
}) {
  const now = useHealthClock();
  const live = useContext(RealtimeContext);
  const unavailable =
    resource.platformStatus !== PlatformStatus.Online ||
    service.isStale ||
    !!(live && live.liveConnectionState !== 'connected') ||
    !resource.probeHealth ||
    now < resource.probeHealth.evaluatedAt ||
    now - resource.probeHealth.evaluatedAt > 180_000;
  const members = serviceHealthMembers(resource, service, platformId).map((member) => ({
    ...member,
    available: member.available && !service.tasks.find((task) => task.id === member.taskId)?.isStale,
  }));
  const summary = groupHealth(members.map(memberHealth), now, service.desiredTaskCount);
  const status = unavailable ? ContainerHealthStatus.Unknown : summary.status;
  const tone = healthTone(status);
  return (
    <HealthCard
      title="Service health"
      resourceName={service.name}
      triggerLabel={`Health: ${healthLabels[status]}`}
      triggerClassName={tone}
      trigger={<Activity className={`size-4 ${tone}`} aria-hidden="true" />}
      status={<HealthLabel status={status} />}
      summary={
        <div className="space-y-3 text-sm">
          <p className="break-words font-medium">{service.name}</p>
          {unavailable ? (
            <p className="text-muted-foreground">Current health evidence is unavailable.</p>
          ) : (
            <>
              <dl className="space-y-2">
                {Object.entries(summary.counts)
                  .filter(([, count]) => count > 0)
                  .map(([state, count]) => (
                    <div key={state} className="flex justify-between gap-4">
                      <dt className="flex items-center gap-2">
                        <Activity
                          className={`size-3.5 shrink-0 ${healthTone(state as ContainerHealthStatus)}`}
                          aria-hidden="true"
                        />
                        {healthLabels[state as ContainerHealthStatus]}
                      </dt>
                      <dd>{count}</dd>
                    </div>
                  ))}
              </dl>
              <p className="text-xs text-muted-foreground">
                {members.length} observed · {service.desiredTaskCount} expected current tasks
              </p>
            </>
          )}
        </div>
      }
      renderDetails={() => <WorkloadHealthMembers members={members} unavailable={unavailable} />}
    />
  );
}

export function WorkloadTaskHealthCard({
  resource,
  task,
  platformId,
}: {
  resource: HealthResource;
  task: SwarmTaskView;
  platformId: string;
}) {
  const member = resource.probeHealth?.members.find(
    (candidate) =>
      candidate.taskId === task.id &&
      candidate.nodeId === task.nodeId &&
      candidate.platformId === platformId &&
      candidate.service === task.serviceId &&
      task.desiredState.toLowerCase() === 'running',
  );
  return member ? (
    <WorkloadMemberHealthCard
      member={task.isStale ? { ...member, available: false } : member}
      platformStatus={resource.platformStatus}
    />
  ) : (
    <UnavailableHealthCard name={task.name} />
  );
}
