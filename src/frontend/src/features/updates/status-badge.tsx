import type { AgentTargetView, AgentUpdatesView, UpdateStatusView } from '@/api/generated/api.types';
import { StateBadge } from '@/components/custom/state-badge';
import { agentStatusLabels } from './status';

const tones: Record<AgentTargetView['status'], string> = {
  Available: 'Warning',
  Current: 'Success',
  Offline: 'Warning',
  Unknown: 'Unknown',
  ManualRequired: 'Warning',
  Incompatible: 'Unknown',
};

export function AgentStatusBadge({ status, count }: { status: AgentTargetView['status']; count?: number }) {
  const label =
    count === undefined
      ? agentStatusLabels[status]
      : status === 'Available'
        ? `${count} update${count === 1 ? '' : 's'} available`
        : `${count} ${agentStatusLabels[status].toLowerCase()}`;
  return <StateBadge value={tones[status]} label={label} />;
}

export function CoreStatusBadge({ status, failed = false }: { status?: UpdateStatusView; failed?: boolean }) {
  if (failed || status?.checkError) return <StateBadge value="Warning" label="Check failed" />;
  if (!status) return <StateBadge value="Unknown" label="Status unavailable" />;
  if (status.operation)
    return (
      <StateBadge
        value={['Failed', 'RecoveryRequired'].includes(status.operation.phase) ? 'Warning' : 'Information'}
        kind="activity"
        label={
          ['Failed', 'RecoveryRequired'].includes(status.operation.phase) ? 'Recovery required' : 'Update in progress'
        }
      />
    );
  if (status.available)
    return (
      <StateBadge
        value="Warning"
        label={status.installationUnavailableReason ? 'Manual update required' : 'Update available'}
      />
    );
  if (status.lastSuccess) return <StateBadge value="Success" label="Up to date" />;
  return <StateBadge value="Unknown" label="Not checked" />;
}

export function AgentStatusSummary({ state, failed = false }: { state?: AgentUpdatesView; failed?: boolean }) {
  if (!state || failed) return <StateBadge value="Unknown" label={failed ? 'Status unavailable' : 'Loading status…'} />;
  return (
    <div className="flex flex-wrap items-center gap-2">
      <span className="text-sm text-muted-foreground">{state.targets.length} registered</span>
      {Object.keys(agentStatusLabels).map((key) => {
        const status = key as AgentTargetView['status'];
        const count = state.targets.filter((target) => target.status === status).length;
        return count ? (
          status === 'Current' && count === state.targets.length ? (
            <StateBadge key={status} value="Success" label="All up to date" />
          ) : (
            <AgentStatusBadge key={status} status={status} count={count} />
          )
        ) : null;
      })}
      {state.checkError && <StateBadge value="Warning" label="Check failed" />}
    </div>
  );
}
