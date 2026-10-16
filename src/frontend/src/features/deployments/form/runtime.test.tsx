import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  ContainerStateStatus,
  DeploymentStatus,
  DeploymentView,
  PlatformStatus,
  ResourceControlState,
} from '@/api/generated/api.types';
import { DeploymentFormComponents } from './index';

const mocks = vi.hoisted(() => ({ read: vi.fn(), live: vi.fn() }));
vi.mock('@/lib/hooks', async (original) => ({
  ...(await original<typeof import('@/lib/hooks')>()),
  useRead: mocks.read,
}));
vi.mock('@/features/docker-resources/containers/hooks/useContainerInfoGroup', () => ({
  useContainerInfoGroup: mocks.live,
}));
vi.mock('@/features/docker-resources/containers/container-info/container-info-table', () => ({
  DeploymentContainerInfoTable: ({ summary }: any) => <div>Metadata: {summary.containerId}</div>,
}));
vi.mock('@/features/docker-resources/containers/container-info/container-logs', () => ({
  DeploymentLogs: () => <div>Container logs</div>,
}));
vi.mock('@/features/docker-resources/containers/container-info/container-inspect', () => ({
  DeploymentInspect: () => <div>Container inspection</div>,
}));
vi.mock('@/features/docker-resources/containers/container-info/container-exec', () => ({
  DeploymentExec: () => <div>Container terminal</div>,
}));
vi.mock('@/features/docker-resources/containers/container-info/container-stats', () => ({
  DeploymentStats: () => <div>Retained statistics</div>,
}));

const tab = DeploymentFormComponents.EditForm!.Tabs!.find((tab) => tab.label === 'Container')!;
const Content = tab.Content;
const deployment = {
  id: 'deployment-id',
  platformId: 'platform-id',
  containerId: 'persisted-container-id',
  dockerContainerId: 'abcdef1234567890',
  status: DeploymentStatus.Degraded,
  platformStatus: PlatformStatus.Online,
  controlState: ResourceControlState.Idle,
  capabilities: { canRead: true, canViewLogs: true, canInspect: true, canOpenTerminal: true },
} as DeploymentView;

function readResult(state = ContainerStateStatus.Running) {
  return {
    data: {
      data: { containerId: deployment.dockerContainerId, platformId: deployment.platformId, state, capabilities: null },
    },
    isLoading: false,
    isFetching: false,
    error: null,
  };
}

beforeEach(() => {
  mocks.read.mockReturnValue(readResult());
  mocks.live.mockReturnValue({});
});

describe('Deployment diagnostics', () => {
  it('keeps a degraded Deployment accessible using workload permissions', () => {
    expect(tab.disabled).toBeUndefined();
    render(<Content resource={deployment} />);
    expect(screen.getByText('Container logs')).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Inspect' })).toBeEnabled();
    expect(screen.getByRole('tab', { name: 'Terminal' })).toBeEnabled();
    expect(mocks.read).toHaveBeenCalledWith('getDeploymentContainerInfo', { id: deployment.id }, { enabled: true });
    expect(mocks.live).toHaveBeenCalledWith(undefined, undefined);
  });

  it.each([ContainerStateStatus.Exited, ContainerStateStatus.Paused, ContainerStateStatus.Restarting])(
    'keeps logs and inspect accessible for %s but prevents terminal access',
    (state) => {
      mocks.read.mockReturnValue(readResult(state));
      render(<Content resource={{ ...deployment, status: DeploymentStatus.Healthy }} />);
      expect(screen.getByRole('tab', { name: 'Logs' })).toBeEnabled();
      expect(screen.getByRole('tab', { name: 'Inspect' })).toBeEnabled();
      expect(screen.getByRole('tab', { name: 'Terminal' })).toBeDisabled();
    },
  );

  it('shows retained statistics while offline and disables live actions', () => {
    render(<Content resource={{ ...deployment, platformStatus: PlatformStatus.Offline }} />);
    expect(screen.getByText('Live diagnostics unavailable')).toBeInTheDocument();
    expect(screen.getByText('Retained statistics')).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Terminal' })).toBeDisabled();
    expect(screen.getByRole('tab', { name: 'Logs' })).toBeDisabled();
  });

  it('distinguishes never deployed from missing runtime data', () => {
    const { rerender } = render(
      <Content resource={{ ...deployment, status: DeploymentStatus.Created, dockerContainerId: null }} />,
    );
    expect(screen.getByText('Not deployed')).toBeInTheDocument();
    rerender(<Content resource={{ ...deployment, dockerContainerId: null }} />);
    expect(screen.getByText('Container unavailable')).toBeInTheDocument();
  });

  it('does not reuse metadata or enable the terminal for a replacement container', () => {
    const { rerender } = render(<Content resource={deployment} />);
    rerender(<Content resource={{ ...deployment, dockerContainerId: 'replacement-id' }} />);
    expect(screen.queryByText(`Metadata: ${deployment.dockerContainerId}`)).not.toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Terminal' })).toBeDisabled();
  });

  it('removes cached metadata after permission denial', () => {
    mocks.read.mockReturnValue({ ...readResult(), error: { status: 403 } });
    render(<Content resource={deployment} />);
    expect(screen.getByText(/do not have permission/)).toBeInTheDocument();
    expect(screen.queryByRole('tab')).not.toBeInTheDocument();
    expect(screen.queryByText(/Metadata:/)).not.toBeInTheDocument();
  });

  it('keeps statistics available when the linked container has been removed', () => {
    mocks.read.mockReturnValue({ ...readResult(), error: { status: 404 } });
    render(<Content resource={deployment} />);
    expect(screen.getByText('Retained statistics')).toBeInTheDocument();
    expect(screen.queryByText(/Metadata:/)).not.toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Terminal' })).toBeDisabled();
  });

  it('lets logs and inspect report their own availability after a metadata failure', () => {
    mocks.read.mockReturnValue({ ...readResult(), error: { status: 500 } });
    render(<Content resource={deployment} />);
    expect(screen.getByRole('tab', { name: 'Logs' })).toBeEnabled();
    expect(screen.getByRole('tab', { name: 'Inspect' })).toBeEnabled();
    expect(screen.getByRole('tab', { name: 'Terminal' })).toBeDisabled();
  });

  it('disables the terminal during Deployment operations', () => {
    render(<Content resource={{ ...deployment, controlState: ResourceControlState.Processing }} />);
    expect(screen.getByRole('tab', { name: 'Terminal' })).toBeDisabled();
    expect(screen.getByRole('tab', { name: 'Logs' })).toBeEnabled();
  });

  it('uses the persisted container identity and rejects stale live state', () => {
    const result = readResult();
    mocks.read.mockReturnValue({ ...result, data: { data: { ...result.data.data, capabilities: { canRead: true } } } });
    mocks.live.mockReturnValue({
      containerInfo: {
        id: deployment.dockerContainerId,
        state: ContainerStateStatus.Running,
        projectionStaleSince: 123,
        platformId: deployment.platformId,
      },
    });
    render(<Content resource={deployment} />);
    expect(mocks.live).toHaveBeenCalledWith(deployment.containerId, deployment.platformId);
    expect(screen.getByRole('tab', { name: 'Terminal' })).toBeDisabled();
  });

  it('preserves separate diagnostic permissions', () => {
    render(
      <Content
        resource={{
          ...deployment,
          capabilities: { ...deployment.capabilities!, canOpenTerminal: false, canViewLogs: false, canInspect: false },
        }}
      />,
    );
    expect(screen.getByRole('tab', { name: 'Terminal' })).toBeDisabled();
    expect(screen.getByRole('tab', { name: 'Logs' })).toBeDisabled();
    expect(screen.getByRole('tab', { name: 'Inspect' })).toBeDisabled();
    expect(screen.getByText('Retained statistics')).toBeInTheDocument();
  });
});
