import { useContext, useState } from 'react';
import { Activity } from 'lucide-react';
import {
  ContainerHealthStatus,
  PlatformStatus,
  type HealthMember,
  type ContainerRuntimeView,
} from '@/api/generated/api.types';
import { HealthCard } from '@/components/custom/health-card';
import {
  currentHealth,
  HealthLabel,
  healthLabels,
  healthTone,
  healthTime,
  useHealthClock,
  inactiveHealthCheckMessage,
} from '@/components/custom/health-status';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { RealtimeContext } from '@/lib/context/realtime-context';
import { useContainerInfoGroup } from '@/features/docker-resources/containers/hooks/useContainerInfoGroup';
import { ContainerHealth } from '@/features/docker-resources/containers/container-info/container-health';
import { requestErrorMessage, requestErrorStatus } from '@/lib/request-error';
import { type HealthResource, memberHealth, workloadHealthStatus, workloadHealthCoverageFresh } from './health';
import type { HealthResourceKind } from './table';

const resourceLabels = { Deployment: 'Deployment', Stack: 'Stack', SwarmService: 'Service' };
const countLabels = {
  healthy: 'Healthy',
  unhealthy: 'Unhealthy',
  starting: 'Starting',
  noCheck: 'No check',
  disabled: 'Disabled',
  notRunning: 'Not running',
  unknown: 'Unknown',
  completed: 'Completed jobs',
};
const countStatuses: Record<keyof typeof countLabels, ContainerHealthStatus> = {
  healthy: ContainerHealthStatus.Healthy,
  unhealthy: ContainerHealthStatus.Unhealthy,
  starting: ContainerHealthStatus.Starting,
  noCheck: ContainerHealthStatus.NoCheck,
  disabled: ContainerHealthStatus.Disabled,
  notRunning: ContainerHealthStatus.NotRunning,
  unknown: ContainerHealthStatus.Unknown,
  completed: ContainerHealthStatus.NotRunning,
};
const attentionLabels: Record<string, string> = {
  Unhealthy: 'Failing checks',
  RuntimeMissing: 'Missing required instances',
  UnexpectedExit: 'Unexpected inactivity',
  RuntimeUnavailable: 'Runtime unavailable',
  ObservationStale: 'Incomplete observations',
  OperationFailed: 'Operation failed',
  OwnershipConflict: 'Ownership conflict',
};

export function WorkloadHealthCard({
  resource,
  kind,
  triggerVariant,
}: {
  resource: HealthResource;
  kind: HealthResourceKind;
  triggerVariant?: 'inline' | 'icon';
}) {
  const now = useHealthClock();
  const live = useContext(RealtimeContext);
  const disconnected = live && live.liveConnectionState !== 'connected';
  const status = disconnected ? ContainerHealthStatus.Unknown : workloadHealthStatus(resource, now);
  const tone = healthTone(status);
  return (
    <HealthCard
      title={`${resourceLabels[kind]} health`}
      resourceName={resource.name}
      triggerLabel={`Health: ${healthLabels[status]}`}
      triggerVariant={triggerVariant}
      triggerClassName={tone}
      trigger={<Activity className={`size-4 ${tone}`} aria-hidden="true" />}
      status={<HealthLabel status={status} />}
      summary={
        <HealthSummary resource={resource} unavailable={!!disconnected || status === ContainerHealthStatus.Unknown} />
      }
      renderDetails={() => (
        <WorkloadHealthDetails
          key={resource.id}
          resource={resource}
          unavailable={!!disconnected || status === ContainerHealthStatus.Unknown}
        />
      )}
    />
  );
}

function HealthSummary({ resource, unavailable }: { resource: HealthResource; unavailable: boolean }) {
  const health = resource.probeHealth;
  const now = useHealthClock();
  const coverageFresh = health && workloadHealthCoverageFresh(health, now);
  const attention =
    health?.attention.filter(
      (reason) => reason !== 'Unhealthy' || workloadHealthStatus(resource, now) === ContainerHealthStatus.Unhealthy,
    ) ?? [];
  return (
    <div className="space-y-3 text-sm">
      <p>
        {unavailable
          ? 'Current health evidence is unavailable. Any observations below are last known.'
          : !coverageFresh
            ? 'Some observations have expired. Counts reflect the last assessment.'
            : (health?.reason ?? 'Health has not been observed.')}
      </p>
      {health && (
        <>
          <dl className="space-y-2">
            {Object.entries(health.counts)
              .filter(([, count]) => count > 0)
              .map(([key, count]) => (
                <div className="flex justify-between gap-4" key={key}>
                  <dt className="flex items-center gap-2 text-muted-foreground">
                    <Activity
                      className={`size-3.5 shrink-0 ${healthTone(countStatuses[key as keyof typeof countLabels])}`}
                      aria-hidden="true"
                    />
                    {countLabels[key as keyof typeof countLabels]}
                  </dt>
                  <dd className="tabular-nums">{count}</dd>
                </div>
              ))}
            {health.missing != null && health.missing > 0 && (
              <div className="flex justify-between gap-4">
                <dt>Missing instances</dt>
                <dd>{health.missing}</dd>
              </div>
            )}
          </dl>
          <p className="border-t pt-3 text-xs text-muted-foreground">
            {health.expected == null
              ? 'Expected membership is unknown.'
              : `${health.observed} observed · ${health.expected} expected`}
            {' · '}
            {health.coverageComplete && coverageFresh && !unavailable ? 'Complete coverage' : 'Partial coverage'}
          </p>
          {attention.length > 0 && !unavailable && (
            <p className="text-xs text-muted-foreground">
              {attention.map((reason) => attentionLabels[reason]).join(' · ')}
            </p>
          )}
          <p className="text-xs text-muted-foreground">Observed {healthTime(health.observedAt)}</p>
        </>
      )}
    </div>
  );
}

function WorkloadHealthDetails({ resource, unavailable }: { resource: HealthResource; unavailable: boolean }) {
  const health = resource.probeHealth;
  const members = health?.members ?? [];
  const now = useHealthClock();
  const coverageNotice = unavailable
    ? 'Current health evidence is unavailable.'
    : !health || !workloadHealthCoverageFresh(health, now)
      ? 'Some health observations are unavailable or outdated.'
      : health.missing
        ? `${health.missing} required ${health.missing === 1 ? 'instance is' : 'instances are'} missing.`
        : !health.coverageComplete || health.expected == null
          ? 'Coverage is partial; some expected containers may be missing.'
          : undefined;
  // Keep workload-level failures that cannot be explained by the selected container.
  const attention = unavailable
    ? []
    : (health?.attention.filter((reason) =>
        ['OperationFailed', 'UnexpectedExit', 'OwnershipConflict'].includes(reason),
      ) ?? []);
  const notice = [coverageNotice, ...attention.map((reason) => attentionLabels[reason])].filter(Boolean).join(' · ');
  return (
    <div className="space-y-5">
      {notice && <p className="text-xs text-muted-foreground">{notice}</p>}
      {resource.probeHealth?.membersTruncated && (
        <p className="text-xs text-muted-foreground">Showing the first 200 containers, with failures first.</p>
      )}
      <WorkloadHealthMembers members={members} unavailable={unavailable} />
    </div>
  );
}

export function WorkloadHealthMembers({ members, unavailable }: { members: HealthMember[]; unavailable: boolean }) {
  const [selected, setSelected] = useState<string>();
  const now = useHealthClock();
  const member = members.find((m) => m.containerId === selected) ?? members.find((m) => !!m.containerId);
  return (
    <div className="space-y-5">
      {members.length > 1 && (
        <Select value={member?.containerId ?? ''} onValueChange={setSelected}>
          <SelectTrigger aria-label="Container" className="w-full min-w-0 [&>span]:min-w-0">
            <SelectValue placeholder="Select a container" />
          </SelectTrigger>
          <SelectContent>
            {members.map((m, index) => {
              const status = unavailable ? ContainerHealthStatus.Unknown : currentHealth(memberHealth(m), now);
              return (
                <SelectItem
                  key={m.containerId ?? `unavailable:${index}`}
                  value={m.containerId ?? `unavailable:${index}`}
                  disabled={!m.containerId}>
                  <span className="flex min-w-0 items-center gap-2">
                    <Activity className={`size-4 shrink-0 ${healthTone(status)}`} aria-hidden="true" />
                    <span className="truncate">{m.name || m.service || 'Container'}</span>
                    <span className="sr-only"> — {healthLabels[status]}</span>
                    {m.nodeId && <span className="truncate text-xs text-muted-foreground">Node {m.nodeId}</span>}
                  </span>
                </SelectItem>
              );
            })}
          </SelectContent>
        </Select>
      )}
      {member ? (
        <MemberDetails
          key={`${member.platformId}:${member.nodeId}:${member.containerId}`}
          member={member}
          unavailable={unavailable}
        />
      ) : (
        <p className="text-sm text-muted-foreground">No current container is available for health history.</p>
      )}
    </div>
  );
}

function MemberDetails({ member, unavailable }: { member: HealthMember; unavailable: boolean }) {
  const { containerInfo, error, isLoading } = useContainerInfoGroup(member.containerId ?? undefined, member.platformId);
  if (error)
    return (
      <p role="status" className="text-sm text-muted-foreground">
        {requestErrorStatus(error) === 403
          ? 'Container health history requires access to its Platform.'
          : requestErrorMessage(error)}
      </p>
    );
  if (isLoading)
    return (
      <p role="status" className="text-sm text-muted-foreground">
        Loading container health…
      </p>
    );
  if (
    !containerInfo ||
    containerInfo.id !== member.dockerContainerId ||
    containerInfo.platformId !== member.platformId ||
    // Persisted UUID identifies the exact runtime scope, including the manager's
    // platform-scoped containers whose Docker node ID is intentionally absent.
    containerInfo.resourceId !== member.containerId
  )
    return (
      <p className="text-sm text-muted-foreground">
        This container is no longer available. Current workload membership will update automatically.
      </p>
    );
  return <ContainerHealth resource={unavailable ? { ...containerInfo, projectionStaleSince: 1 } : containerInfo} />;
}

export function WorkloadMemberHealthCard({
  member,
  platformStatus,
}: {
  member: HealthMember;
  platformStatus: PlatformStatus;
}) {
  const now = useHealthClock();
  const live = useContext(RealtimeContext);
  const unavailable = platformStatus !== PlatformStatus.Online || !!(live && live.liveConnectionState !== 'connected');
  const status = unavailable ? ContainerHealthStatus.Unknown : currentHealth(memberHealth(member), now);
  const tone = healthTone(status);
  return (
    <HealthCard
      title="Container health"
      resourceName={member.name}
      triggerLabel={`Health: ${healthLabels[status]}`}
      triggerClassName={tone}
      trigger={<Activity className={`size-4 ${tone}`} aria-hidden="true" />}
      status={<HealthLabel status={status} />}
      summary={
        <div className="space-y-2 text-sm">
          <p className="break-all">{member.name}</p>
          {inactiveHealthCheckMessage(status) && (
            <p className="text-muted-foreground">{inactiveHealthCheckMessage(status)}</p>
          )}
          <p className="text-xs text-muted-foreground">Observed {healthTime(member.observation?.observedAt)}</p>
          {member.nodeId && <p className="text-xs text-muted-foreground">Node {member.nodeId}</p>}
        </div>
      }
      renderDetails={member.containerId ? () => <MemberDetails member={member} unavailable={unavailable} /> : undefined}
    />
  );
}

/** Match persisted identity; never use a name or statistics sample to resolve diagnostics. */
export function WorkloadContainerHealthCard({
  resource,
  container,
}: {
  resource: HealthResource;
  container: Pick<ContainerRuntimeView, 'id' | 'platformId' | 'dockerNodeId' | 'name'>;
}) {
  const member = resource.probeHealth?.members.find(
    (candidate) =>
      candidate.platformId === container.platformId &&
      candidate.dockerContainerId === container.id &&
      (candidate.nodeId ?? null) === (container.dockerNodeId ?? null),
  );
  if (member) return <WorkloadMemberHealthCard member={member} platformStatus={resource.platformStatus} />;
  return <UnavailableHealthCard name={container.name} />;
}

export function UnavailableHealthCard({ name }: { name: string }) {
  return (
    <HealthCard
      title="Container health"
      resourceName={name}
      triggerLabel="Health: Unknown"
      trigger={<Activity className="size-4 text-muted-foreground" aria-hidden="true" />}
      status={<HealthLabel status={ContainerHealthStatus.Unknown} />}
      summary={
        <p className="text-sm text-muted-foreground">Current health evidence is unavailable for this container.</p>
      }
    />
  );
}
