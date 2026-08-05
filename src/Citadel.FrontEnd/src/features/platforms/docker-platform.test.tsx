import { PlatformConnectorType, PlatformStatus, PlatformType, PlatformView } from '@/api/generated/api.types';
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router';
import { DockerPlatform } from './docker-platform';

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

    render(
      <MemoryRouter>
        <DockerPlatform platform={platform} actions={{}} />
      </MemoryRouter>,
    );

    expect(screen.getByLabelText('Docker Swarm')).toBeVisible();
    expect(screen.getByText('3 nodes')).toBeVisible();
    expect(screen.getByText('1 manager')).toBeVisible();
    expect(screen.getByText('6 services')).toBeVisible();
    expect(screen.getByText('10 running tasks')).toBeVisible();
    expect(screen.getByRole('link', { name: '3 nodes' })).toHaveAttribute('href', `/platforms/${platform.id}/nodes`);
    expect(screen.getByRole('link', { name: '1 manager' })).toHaveAttribute('href', `/platforms/${platform.id}/nodes`);
    expect(screen.getByRole('link', { name: '6 services' })).toHaveAttribute(
      'href',
      `/platforms/${platform.id}/services`,
    );
    expect(screen.getByRole('link', { name: '10 running tasks' })).toHaveAttribute(
      'href',
      `/platforms/${platform.id}/tasks`,
    );
    expect(screen.getByRole('link', { name: 'Running tasks' })).toHaveAttribute(
      'href',
      `/platforms/${platform.id}/tasks`,
    );
    expect(screen.getByRole('region', { name: 'Swarm workloads' })).toBeVisible();
    expect(screen.getByRole('region', { name: 'Connected manager utilization' })).toBeVisible();
    expect(screen.queryByText('2 containers')).not.toBeInTheDocument();
  });
});
