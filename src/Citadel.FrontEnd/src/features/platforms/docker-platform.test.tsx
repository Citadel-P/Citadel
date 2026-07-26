import { PlatformConnectorType, PlatformStatus, PlatformView } from '@/api/generated/api.types';
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router';
import { DockerPlatform } from './docker-platform';

const createPlatform = (diskUsage: number | null): PlatformView =>
  ({
    id: 'platform-1',
    name: 'Platform',
    status: PlatformStatus.Online,
    connectorType: PlatformConnectorType.Local,
    serverVersion: '29.0.0',
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
});
