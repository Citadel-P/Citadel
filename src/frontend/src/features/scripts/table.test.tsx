import { ScriptRunStatus, ScriptRunTrigger, AuthorizedScript, ResourceControlState } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { screen } from '@testing-library/react';
import { ScriptsTable } from './table';

const createScript = (enabled: boolean): AuthorizedScript => ({
  executor: { kind: 'Core' },
  capabilities: { canRead: true, canWrite: true, canExecute: true },
  id: '019fbf2c-1c80-790f-a50a-11bbec0241d3',
  name: enabled ? 'Enabled script' : 'Disabled script',
  enabled,
  description: null,
  code: '',
  defaultArgsJson: '{}',
  scheduleCron: null,
  scheduleTimeZone: 'UTC',
  webhook: null,
  timeoutSeconds: 300,
  alertOnFailure: false,
  runAsActorId: 'actor-id',
  lastScheduledRunAt: null,
  currentRunId: null,
  rowVersion: 1,
  createdByActorId: 'actor-id',
  createdAt: '2026-09-29T00:00:00Z',
  updatedAt: '2026-09-29T00:00:00Z',
  scheduleEnabled: false,
  controlState: ResourceControlState.Idle,
  latestRun: {
    attemptId: null,
    executorId: null,
    cleanupPending: false,
    id: 'run-id',
    scriptId: 'script-id',
    scriptName: 'Script',
    trigger: ScriptRunTrigger.Manual,
    runAsActorId: 'actor-id',
    triggeredByActorId: null,
    argsJson: '{}',
    codeSnapshot: null,
    codeHash: '',
    timeoutSeconds: 300,
    startedAt: null,
    finishedAt: null,
    durationMs: null,
    exitCode: 0,
    logs: null,
    errorMessage: null,
    status: ScriptRunStatus.Succeeded,
    queuedAt: '2026-08-02T00:00:00Z',
  },
  tags: [],
});

describe('ScriptsTable', () => {
  it('shows the disabled state instead of a successful last-run status for a disabled script', async () => {
    const { user } = renderCitadel(<ScriptsTable items={[createScript(false)]} actions={{}} isLoading={false} />);

    const indicator = screen.getByRole('link', { name: 'Disabled script' }).previousElementSibling;

    expect(indicator).toHaveClass('bg-gray-500');
    expect(indicator).not.toHaveClass('bg-green-500');
    await user.hover(indicator!);
    expect(await screen.findByRole('tooltip')).toHaveTextContent('Disabled');
  });

  it('shows Enabled instead of the last-run result for an enabled script', async () => {
    const { user } = renderCitadel(<ScriptsTable items={[createScript(true)]} actions={{}} isLoading={false} />);

    const indicator = screen.getByRole('link', { name: 'Enabled script' }).previousElementSibling;

    expect(indicator).toHaveClass('bg-green-500');
    await user.hover(indicator!);
    expect(await screen.findByRole('tooltip')).toHaveTextContent('Enabled');
  });
});
