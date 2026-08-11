import {
  PlatformConnectorType,
  PlatformStatus,
  PlatformType,
  PlatformView,
  SwarmQuorumState,
} from '@/api/generated/api.types';
import { render, screen, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router';
import { DockerPlatform } from './docker-platform';

const { useSwarmOverviewMock } = vi.hoisted(() => ({
  useSwarmOverviewMock: vi.fn(() => ({ overview: undefined as unknown })),
}));

vi.mock('@/features/swarm/hooks/useSwarmOverview', () => ({
  useSwarmOverview: useSwarmOverviewMock,
}));

beforeEach(() => {
  useSwarmOverviewMock.mockReturnValue({ overview: undefined });
});

const createPlatform = (diskUsage: number | null): PlatformView =>
  ({
    id: 'platform-1',
    name: 'Platform',
    type: PlatformType.Docker,
    status: PlatformStatus.Online,
    connectorType: PlatformConnectorType.Local,
    serverVersion: '29.0.0',
    deploymentStatusCounts: {
      total: 4,
      healthy: 2,
      degraded: 1,
      failed: 0,
      stopped: 1,
      paused: 0,
      inProgress: 0,
      unknown: 0,
    },
    stackStatusCounts: {
      total: 5,
      healthy: 2,
      degraded: 1,
      failed: 0,
      stopped: 1,
      paused: 1,
      inProgress: 0,
      unknown: 0,
    },
    swarmServiceStatusCounts: {
      total: 0,
      healthy: 0,
      degraded: 0,
      failed: 0,
      stopped: 0,
      paused: 0,
      inProgress: 0,
      unknown: 0,
    },
    platformDescriptor: {
      operatingSystem: 'Linux',
      containerCount: 1,
      containersRunning: 1,
      containersStopped: 0,
      containersPaused: 0,
    },
    stats: [
      {
        created: 1,
        cpuUsage: 10,
        memoryUsage: 20,
        diskUsage,
        diskUsedBytes: diskUsage == null ? null : diskUsage,
        diskTotalBytes: diskUsage == null ? null : 100,
      },
    ],
    tags: [],
  }) as PlatformView;

describe('DockerPlatform disk usage', () => {
  it('updates the disk percentage from streamed platform snapshots', () => {
    const { rerender } = render(
      <MemoryRouter>
        <DockerPlatform platform={createPlatform(42.5)} actions={{}} />
      </MemoryRouter>,
    );

    expect(screen.getByText('42.5 %')).toBeVisible();
    expect(screen.getByRole('region', { name: 'Platform workloads' })).toBeVisible();
    expect(screen.getByRole('region', { name: 'Platform utilization' })).toBeVisible();
    expect(screen.getAllByLabelText('Healthy: 2')).toHaveLength(2);
    expect(screen.getAllByLabelText('Degraded: 1')).toHaveLength(2);
    expect(screen.getAllByLabelText('Stopped: 1')).toHaveLength(2);
    expect(screen.getByLabelText('Paused: 1')).toBeVisible();
    expect(screen.getByRole('progressbar', { name: 'CPU usage' })).toHaveAttribute('aria-valuenow', '10');
    expect(screen.getByRole('progressbar', { name: 'RAM usage' })).toHaveAttribute('aria-valuenow', '20');
    expect(screen.getByRole('progressbar', { name: 'Disk usage' })).toHaveAttribute('aria-valuenow', '42.5');

    rerender(
      <MemoryRouter>
        <DockerPlatform platform={createPlatform(43.25)} actions={{}} />
      </MemoryRouter>,
    );

    expect(screen.getByText('43.25 %')).toBeVisible();
    expect(screen.queryByText('42.5 %')).not.toBeInTheDocument();
  });

  it('shows unavailable when the streamed disk metric is absent', () => {
    render(
      <MemoryRouter>
        <DockerPlatform platform={createPlatform(null)} actions={{}} />
      </MemoryRouter>,
    );

    expect(screen.getByText('N/A')).toBeVisible();
    expect(screen.getByRole('progressbar', { name: 'Disk usage' })).not.toHaveAttribute('aria-valuenow');
  });

  it('shows unavailable when the streamed disk percentage is invalid', () => {
    render(
      <MemoryRouter>
        <DockerPlatform platform={createPlatform(101)} actions={{}} />
      </MemoryRouter>,
    );

    expect(screen.getByText('N/A')).toBeVisible();
    expect(screen.queryByText('101 %')).not.toBeInTheDocument();
  });

  it('renders cluster inventory and a distinct icon for a Swarm platform', () => {
    const platform = createPlatform(42.5);
    platform.type = PlatformType.DockerSwarm;
    platform.volumeCount = 4;
    platform.networkCount = 5;
    platform.imageCount = 7;
    platform.platformDescriptor = {
      $type: 'DockerSwarm',
      operatingSystem: 'Linux',
      nodes: 3,
      managers: 1,
      serviceCount: 6,
      runningTaskCount: 10,
      containerCount: 2,
      containersRunning: 2,
      containersStopped: 0,
      containersPaused: 0,
    } as PlatformView['platformDescriptor'];
    platform.swarmServiceStatusCounts = {
      total: 4,
      healthy: 1,
      degraded: 1,
      failed: 1,
      stopped: 0,
      paused: 0,
      inProgress: 1,
      unknown: 0,
    };
    useSwarmOverviewMock.mockReturnValue({
      overview: {
        nodeCount: 3,
        managerCount: 1,
        quorum: {
          state: SwarmQuorumState.Healthy,
          reachableManagers: 1,
          requiredManagers: 1,
          hasLeader: true,
        },
        serviceCount: 6,
        runningTaskCount: 10,
        serviceStatusCounts: {
          total: 6,
          healthy: 3,
          degraded: 1,
          failed: 1,
          stopped: 1,
          paused: 0,
          inProgress: 0,
          unknown: 0,
        },
      },
    });

    render(
      <MemoryRouter>
        <DockerPlatform platform={platform} actions={{}} />
      </MemoryRouter>,
    );

    expect(screen.getByLabelText('Docker Swarm')).toBeVisible();
    expect(screen.getByText('3 nodes')).toBeVisible();
    expect(screen.getByRole('status', { name: 'Swarm quorum healthy, 1/1 managers reachable' })).toBeVisible();
    expect(screen.queryByText('1 manager')).not.toBeInTheDocument();
    expect(screen.getByText('4 services')).toBeVisible();
    expect(screen.getByText('10 running tasks')).toBeVisible();
    expect(screen.getByText('2 containers')).toBeVisible();
    expect(screen.getByText('4 volumes')).toBeVisible();
    expect(screen.getByText('5 networks')).toBeVisible();
    expect(screen.getByText('7 images')).toBeVisible();
    expect(screen.getByRole('link', { name: '3 nodes' })).toHaveAttribute('href', `/platforms/${platform.id}/nodes`);
    expect(screen.getByRole('link', { name: '4 services' })).toHaveAttribute(
      'href',
      `/swarm-services?platformId=${platform.id}`,
    );
    expect(screen.getByRole('link', { name: '10 running tasks' })).toHaveAttribute(
      'href',
      `/platforms/${platform.id}/tasks`,
    );
    expect(screen.getByRole('link', { name: '2 containers' })).toHaveAttribute(
      'href',
      `/platforms/${platform.id}/containers`,
    );
    expect(screen.getByRole('link', { name: '4 volumes' })).toHaveAttribute(
      'href',
      `/platforms/${platform.id}/volumes`,
    );
    expect(screen.getByRole('link', { name: '5 networks' })).toHaveAttribute(
      'href',
      `/platforms/${platform.id}/networks`,
    );
    expect(screen.getByRole('link', { name: '7 images' })).toHaveAttribute('href', `/platforms/${platform.id}/images`);
    const servicesMetric = screen.getByRole('link', { name: 'Services' }).parentElement!;
    expect(screen.getByRole('link', { name: 'Services' })).toHaveAttribute(
      'href',
      `/swarm-services?platformId=${platform.id}`,
    );
    expect(within(servicesMetric).getByLabelText('Healthy: 1')).toBeVisible();
    expect(within(servicesMetric).getByLabelText('Degraded: 1')).toBeVisible();
    expect(within(servicesMetric).getByLabelText('Failed: 1')).toBeVisible();
    expect(within(servicesMetric).getByLabelText('In progress: 1')).toBeVisible();
    expect(screen.getByRole('region', { name: 'Swarm workloads' })).toBeVisible();
    expect(screen.getByRole('region', { name: 'Connected manager utilization' })).toBeVisible();
  });
});
