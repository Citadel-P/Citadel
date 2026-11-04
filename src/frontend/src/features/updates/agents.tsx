import type { AgentTargetView, AgentUpdatesView } from '@/api/generated/api.types';
import { MultiSelect } from '@/components/ui/multi-select';
import { AlertMessage } from '@/components/custom/alert-message';
import { agentStatusLabels } from './status';
import { AgentStatusBadge, AgentStatusSummary } from './status-badge';

export function agentOptions(targets: AgentTargetView[], scheduled: boolean) {
  return targets.map((target) => ({
    value: target.id,
    label: `${target.name}${target.nodes > 1 ? ` · ${target.nodes} nodes` : ''} · ${target.installedVersion ?? 'Unknown'}${target.status === 'Available' ? ` → ${target.availableVersion}` : ''}`,
    disabled: scheduled ? !target.supported : target.status !== 'Available',
    disabledReason: target.reason ?? agentStatusLabels[target.status],
    trailing: <AgentStatusBadge status={target.status} />,
  }));
}

export function AgentUpdates({
  state,
  failed,
  allowed,
  busy,
  selected,
  onSelect,
}: {
  state?: AgentUpdatesView;
  failed: boolean;
  allowed: boolean;
  busy: boolean;
  selected: string[];
  onSelect: (value: string[]) => void;
}) {
  if (!state) return <p role="status">{failed ? 'Could not load Agent updates.' : 'Loading Agent updates…'}</p>;
  return (
    <div className="space-y-3">
      <AgentStatusSummary state={state} failed={failed} />
      {state.checkError && <AlertMessage type="warning">{state.checkError}</AlertMessage>}
      {state.targets.some((target) => target.status === 'Available') && (
        <>
          <span className="text-sm" id="manual-agent-selection">
            Agents to update now
          </span>
          <div className="max-w-100">
            <MultiSelect
              aria-labelledby="manual-agent-selection"
              options={agentOptions(state.targets, false)}
              defaultValue={selected}
              onValueChange={onSelect}
              disabled={busy || !allowed || failed}
              placeholder="All available Agents"
              hideSelectAll={false}
            />
          </div>
          <p className="text-xs text-muted-foreground">
            Leave empty to review all available Agents. Manual selection does not change your schedule.
          </p>
        </>
      )}
      {state.targets.some((t) => t.status === 'ManualRequired') && (
        <a
          className="text-sm underline text-muted-foreground"
          href="https://docs.citadelplane.com/operations/upgrade-and-rollback/"
          target="_blank"
          rel="noreferrer">
          Manual Agent update guidance
        </a>
      )}
    </div>
  );
}
