vi.mock('@/lib/use-profile-date-time', () => ({ useProfileDateTimeFormatter: () => String }));

import { fireEvent, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { MemoryRouter } from 'react-router';
import {
  CheckConfiguration,
  HealthStatus,
  PlatformStatus,
  WorkloadHealthStatus,
  type HealthMember,
} from '@/api/generated/api.types';
import { createWorkloadHealth } from '@/test/factories/resources';
import { WorkloadHealthCard, WorkloadHealthMembers } from './card';
const containerRead = vi.fn<
  (...args: unknown[]) => {
    isLoading: boolean;
    containerInfo?: { id: string; resourceId: string; platformId: string; dockerNodeId?: string };
    error?: unknown;
  }
>(() => ({ isLoading: false }));
vi.mock('@/features/docker-resources/containers/hooks/useContainerInfoGroup', () => ({
  useContainerInfoGroup: (...args: unknown[]) => containerRead(...(args as [])),
}));
vi.mock('@/features/docker-resources/containers/container-info/container-health', () => ({
  ContainerHealth: () => <div>Shared container diagnostics</div>,
}));
const resource = {
  id: 'owner',
  name: 'web',
  platformStatus: PlatformStatus.Online,
  probeHealth: createWorkloadHealth({
    evaluatedAt: Date.now(),
    observedAt: Date.now(),
    status: WorkloadHealthStatus.Healthy,
    counts: { healthy: 1, unhealthy: 0, starting: 0, noCheck: 0, disabled: 0, notRunning: 0, unknown: 0, completed: 0 },
    expected: 1,
    observed: 1,
    missing: 0,
    coverageComplete: true,
    reason: 'All required containers have fresh passing checks.',
  }),
};
describe('shared workload card', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    containerRead.mockReturnValue({ isLoading: false });
  });

  it('opens manager diagnostics by persisted identity and rejects a different container scope', () => {
    const member: HealthMember = {
      containerId: 'manager-resource',
      dockerContainerId: 'docker-id',
      platformId: 'platform',
      nodeId: 'manager-node',
      taskId: 'task',
      name: 'web.1',
      service: 'web',
      state: 'Running',
      available: true,
      status: HealthStatus.Healthy,
    };
    const containerInfo = { id: 'docker-id', resourceId: 'manager-resource', platformId: 'platform' };
    containerRead.mockReturnValue({ isLoading: false, containerInfo });
    const view = render(<WorkloadHealthMembers members={[member]} unavailable={false} />);
    expect(screen.getByText('Shared container diagnostics')).toBeVisible();
    containerRead.mockReturnValue({
      isLoading: false,
      containerInfo: { ...containerInfo, resourceId: 'other-node-resource' },
    });
    view.rerender(<WorkloadHealthMembers members={[member]} unavailable={false} />);
    expect(screen.queryByText('Shared container diagnostics')).not.toBeInTheDocument();
    expect(screen.getByText(/This container is no longer available/)).toBeVisible();
  });

  it('selects exact containers with current status icons and preserves selection across updates', async () => {
    const member: HealthMember = {
      containerId: 'db-id',
      dockerContainerId: 'docker-db',
      platformId: 'platform',
      name: 'wordpress-db-1',
      service: 'db',
      state: 'Running',
      status: HealthStatus.Healthy,
      available: true,
      observation: { configuration: CheckConfiguration.Enabled, observedAt: Date.now() },
    };
    const members = [
      member,
      {
        ...member,
        containerId: 'web-id',
        dockerContainerId: 'docker-web',
        name: 'wordpress-wordpress-1',
        status: HealthStatus.NoCheck,
      },
    ];
    const view = render(<WorkloadHealthMembers members={members} unavailable={false} />);
    expect(containerRead).toHaveBeenLastCalledWith('db-id', 'platform');
    const user = userEvent.setup();
    await user.click(screen.getByRole('combobox', { name: 'Container' }));
    expect(
      within(screen.getByRole('option', { name: /wordpress-db-1/ }))
        .getByText('wordpress-db-1')
        .parentElement?.querySelector('svg'),
    ).toHaveClass('text-emerald-600');
    await user.click(screen.getByRole('option', { name: /wordpress-wordpress-1/ }));
    expect(containerRead).toHaveBeenLastCalledWith('web-id', 'platform');
    expect(screen.getByRole('combobox')).toHaveTextContent('No check');
    view.rerender(<WorkloadHealthMembers members={[...members].reverse()} unavailable />);
    expect(containerRead).toHaveBeenLastCalledWith('web-id', 'platform');
    expect(screen.getByRole('combobox')).toHaveTextContent('Unknown');
    expect(screen.getByRole('combobox').querySelector('svg')).toHaveClass('text-muted-foreground');
  });

  it('keeps partial coverage concise without repeating the aggregate summary in details', async () => {
    render(
      <WorkloadHealthCard
        kind="Stack"
        resource={{ ...resource, probeHealth: { ...resource.probeHealth, expected: null, coverageComplete: false } }}
      />,
    );
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /Health:/ }));
    await user.click(screen.getByRole('button', { name: 'View details' }));
    const dialog = within(screen.getByRole('dialog', { name: 'Health details' }));
    expect(dialog.getByText('Coverage is partial; some expected containers may be missing.')).toBeVisible();
    expect(dialog.queryByText(resource.probeHealth.reason)).not.toBeInTheDocument();
    expect(dialog.queryByText(/Observed|Expected membership/)).not.toBeInTheDocument();
  });

  it.each(['Deployment', 'Stack', 'SwarmService'] as const)(
    'uses an accessible icon and shared dialog for %s',
    async (kind) => {
      render(
        <MemoryRouter>
          <WorkloadHealthCard resource={resource} kind={kind} />
        </MemoryRouter>,
      );
      const trigger = screen.getByRole('button', { name: 'Health: Healthy' });
      expect(trigger.querySelector('svg')).toHaveClass('text-emerald-600');
      fireEvent.mouseOver(trigger);
      expect(containerRead).not.toHaveBeenCalled();
      await userEvent.click(trigger);
      expect(screen.getByRole('dialog')).toBeInTheDocument();
      expect(screen.getByText('1 observed · 1 expected · Complete coverage')).toBeInTheDocument();
      await userEvent.click(screen.getByRole('button', { name: 'View details' }));
      expect(screen.getByText('No current container is available for health history.')).toBeInTheDocument();
    },
  );
});
