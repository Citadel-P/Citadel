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
  if (status.operation && status.operation.component !== 'Database')
    return (
      <StateBadge
        value={['Failed', 'RecoveryRequired'].includes(status.operation.phase) ? 'Warning' : 'Information'}
        kind="activity"
        label={
          ['Failed', 'RecoveryRequired'].includes(status.operation.phase) ? 'Recovery required' : 'Update in progress'
        }
      />
    );
  if (status.available) return <StateBadge value="Warning" label="Update available" />;
  if (status.lastSuccess) return <StateBadge value="Success" label="Up to date" />;
  return <StateBadge value="Unknown" label="Not checked" />;
}

export function AgentStatusSummary({ state, failed = false }: { state?: AgentUpdatesView; failed?: boolean }) {
  if (!state || failed) return <StateBadge value="Unknown" label={failed ? 'Status unavailable' : 'Loading status…'} />;
  if (!state.targets.length) return <span className="text-sm text-muted-foreground">No Agents registered</span>;
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

export function DatabaseStatusBadge({ status }: { status?: UpdateStatusView }) {
  if (!status?.databaseVersion) return <StateBadge value="Unknown" label="Version unavailable" />;
  if (status.operation?.component === 'Database')
    return (
      <StateBadge
        value={['Failed', 'RecoveryRequired'].includes(status.operation.phase) ? 'Warning' : 'Information'}
        label={
          ['Failed', 'RecoveryRequired'].includes(status.operation.phase) ? 'Recovery required' : 'Update in progress'
        }
      />
    );
  if (!status.databaseManaged) {
    if (status.databaseInstallation === 'External')
      return <StateBadge value="Unknown" label="In-app updates unavailable" />;
    if (status.databaseInstallation === 'Bundled')
      return (
        <StateBadge
          value={status.installationUnavailableReason ? 'Unknown' : 'Warning'}
          label={status.installationUnavailableReason ? 'In-app updates not configured' : 'Update blocked'}
        />
      );
    return <StateBadge value="Unknown" label="Status unavailable" />;
  }
  if (status.databaseAvailable) return <StateBadge value="Warning" label="Update available" />;
  return <StateBadge value="Success" label="Up to date" />;
}

export function AgentSummaryBadge({ state, failed }: { state?: AgentUpdatesView; failed?: boolean }) {
  if (!state || failed) return <StateBadge value="Unknown" label="Status unavailable" />;
  if (state.operation && !state.operation.finished)
    return <StateBadge value="Information" label="Update in progress" />;
  if (!state.targets.length) return <StateBadge value="Unknown" label="No Agents registered" />;
  if (state.checkError) return <StateBadge value="Warning" label="Check failed" />;
  const attention = state.targets.filter((target) => target.status !== 'Current').length;
  return attention ? (
    <StateBadge value="Warning" label={`${attention} ${attention === 1 ? 'Agent needs' : 'Agents need'} attention`} />
  ) : (
    <StateBadge value="Success" label="All up to date" />
  );
}
