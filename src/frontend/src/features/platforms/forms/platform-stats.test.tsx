import { createPlatform as platformDefaults } from '@/test/factories/resources';
import { PlatformConnectorType, PlatformStatus, PlatformType, PlatformView } from '@/api/generated/api.types';
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router';
import { useRead } from '@/lib/hooks';
import {
  DiskUsageChart,
  formatPlatformConnector,
  getCurrentDiskUsage,
  normalizePlatformStats,
  PlatformResourceSummary,
  PlatformStatsTab,
} from './platform-stats';

vi.mock('@/features/swarm/hooks/useSwarmOverview', () => ({
  useSwarmOverview: () => ({ overview: undefined, isLoading: false, error: undefined }),
}));

vi.mock('@/lib/hooks', () => ({ useRead: vi.fn() }));

// Preserve the card and summary rendering without depending on SVG layout in jsdom.
vi.mock('@/components/ui/chart', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/components/ui/chart')>()),
  ChartContainer: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));
vi.mock('recharts', async (importOriginal) => ({
  ...(await importOriginal<typeof import('recharts')>()),
  AreaChart: ({ data }: { data: unknown[] }) => <div data-testid="chart" data-samples={JSON.stringify(data)} />,
}));

describe('platform current statistics', () => {
  const historical = {
    created: 60,
    cpuUsage: 0.51,
    memoryUsage: 4.5,
    rxBytes: 10,
    txBytes: 20,
    diskUsage: null,
    diskUsedBytes: null,
    diskTotalBytes: null,
  };
  const current = { ...historical, created: 95, cpuUsage: 14.33 / 12, memoryUsage: 5 };
  const platform = {
    ...platformDefaults(),
    id: 'platform-1',
    status: PlatformStatus.Online,
    memTotal: 16 * 1024 ** 3,
    stats: [current],
  };

  beforeEach(() => {
    vi.mocked(useRead).mockReturnValue({
      data: { data: { stats: [historical] } },
      isLoading: false,
    } as ReturnType<typeof useRead>);
  });

  it('shows the latest CPU and memory on opening the tab, before any realtime update', () => {
    render(<PlatformStatsTab platform={platform} />);

    expect(screen.getByText('1.19%')).toBeVisible();
    expect(screen.getByText('5.00%')).toBeVisible();
    expect(screen.getByText('819.2 MB')).toBeVisible();
    expect(screen.queryByText('0.51%')).not.toBeInTheDocument();
    const samples = JSON.parse(screen.getAllByTestId('chart')[0].getAttribute('data-samples')!);
    expect(samples.map((sample: { created: number }) => sample.created)).toEqual([60, 95]);
  });

  it('updates live readings and discards the previous platform samples when switching platforms', () => {
    const { rerender } = render(<PlatformStatsTab platform={platform} />);
    rerender(<PlatformStatsTab platform={{ ...platform, stats: [{ ...current, created: 110, cpuUsage: 2 }] }} />);
    expect(screen.getByText('2.00%')).toBeVisible();

    rerender(
      <PlatformStatsTab
        platform={{ ...platform, id: 'platform-2', stats: [{ ...current, created: 120, cpuUsage: 3 }] }}
      />,
    );
    expect(screen.getByText('3.00%')).toBeVisible();
    const samples = JSON.parse(screen.getAllByTestId('chart')[0].getAttribute('data-samples')!);
    expect(samples.map((sample: { created: number }) => sample.created)).toEqual([60, 120]);
  });

  it('does not present history as a current reading when the current sample is unavailable or offline', () => {
    const { rerender } = render(<PlatformStatsTab platform={{ ...platform, stats: [] }} />);
    expect(screen.queryByText('0.51%')).not.toBeInTheDocument();
    expect(screen.queryByText('4.50%')).not.toBeInTheDocument();

    rerender(<PlatformStatsTab platform={{ ...platform, status: PlatformStatus.Offline }} />);
    expect(screen.queryByText('1.19%')).not.toBeInTheDocument();
    expect(screen.queryByText('819.2 MB')).not.toBeInTheDocument();
  });
});

describe('platform disk statistics', () => {
  it('describes the connector without presenting a local Core version as an agent', () => {
    expect(
      formatPlatformConnector({
        connectorType: PlatformConnectorType.Local,
        agentVersion: '1.0',
      } as PlatformView),
    ).toBe('Local');
    expect(
      formatPlatformConnector({
        connectorType: PlatformConnectorType.Agent,
        agentVersion: '1.0.0',
      } as PlatformView),
    ).toBe('Agent v1.0.0');
    expect(
      formatPlatformConnector({
        connectorType: PlatformConnectorType.EdgeAgent,
        agentVersion: null,
      } as PlatformView),
    ).toBe('Edge agent');
  });

  it('preserves unavailable disk values while retaining valid zeroes', () => {
    const stats = normalizePlatformStats([
      {
        created: 1,
        rxBytes: 0,
        txBytes: 0,
        cpuUsage: 0,
        memoryUsage: 0,
        diskUsedBytes: null,
        diskTotalBytes: null,
        diskUsage: null,
      },
      {
        created: 2,
        rxBytes: 0,
        txBytes: 0,
        cpuUsage: 0,
        memoryUsage: 0,
        diskUsedBytes: 0,
        diskTotalBytes: 100,
        diskUsage: 0,
      },
    ]);

    expect(stats[0]).toMatchObject({
      diskUsedBytes: null,
      diskTotalBytes: null,
      diskUsage: null,
    });
    expect(stats[1]).toMatchObject({
      diskUsedBytes: 0,
      diskTotalBytes: 100,
      diskUsage: 0,
    });
  });

  it('does not fall back to an older disk sample when the newest is unavailable', () => {
    const platform = {
      ...platformDefaults(),
      status: PlatformStatus.Online,
      stats: [
        {
          created: 2,
          rxBytes: 0,
          txBytes: 0,
          cpuUsage: 0,
          memoryUsage: 0,
          diskUsedBytes: null,
          diskTotalBytes: null,
          diskUsage: null,
        },
        {
          created: 1,
          rxBytes: 0,
          txBytes: 0,
          cpuUsage: 0,
          memoryUsage: 0,
          diskUsedBytes: 75,
          diskTotalBytes: 100,
          diskUsage: 75,
        },
      ],
    } as PlatformView;

    expect(getCurrentDiskUsage(platform)).toBeNull();
  });

  it('treats out-of-range disk percentages as unavailable', () => {
    const [normalized] = normalizePlatformStats([
      {
        created: 1,
        rxBytes: 0,
        txBytes: 0,
        cpuUsage: 0,
        memoryUsage: 0,
        diskUsedBytes: 75,
        diskTotalBytes: 100,
        diskUsage: 101,
      },
    ]);
    const platform = {
      ...platformDefaults(),
      status: PlatformStatus.Online,
      stats: [
        {
          created: 1,
          rxBytes: 0,
          txBytes: 0,
          cpuUsage: 0,
          memoryUsage: 0,
          diskUsedBytes: 75,
          diskTotalBytes: 100,
          diskUsage: -1,
        },
      ],
    } as PlatformView;

    expect(normalized.diskUsage).toBeNull();
    expect(normalized.diskUsedBytes).toBeNull();
    expect(normalized.diskTotalBytes).toBeNull();
    expect(getCurrentDiskUsage(platform)).toBeNull();
  });

  it('derives available bytes from the newest complete sample', () => {
    const platform = {
      ...platformDefaults(),
      status: PlatformStatus.Online,
      stats: [
        {
          created: 2,
          rxBytes: 0,
          txBytes: 0,
          cpuUsage: 0,
          memoryUsage: 0,
          diskUsedBytes: 75,
          diskTotalBytes: 100,
          diskUsage: 75,
        },
      ],
    } as PlatformView;

    expect(getCurrentDiskUsage(platform)).toEqual({
      usagePercent: 75,
      usedBytes: 75,
      totalBytes: 100,
      availableBytes: 25,
    });
  });

  it('renders the current disk summary with its filesystem tooltip', () => {
    const platform = {
      ...platformDefaults(),
      id: 'platform-1',
      status: PlatformStatus.Online,
      stats: [
        {
          created: 2,
          rxBytes: 0,
          txBytes: 0,
          cpuUsage: 0,
          memoryUsage: 0,
          diskUsedBytes: 75,
          diskTotalBytes: 100,
          diskUsage: 75,
        },
      ],
    } as PlatformView;

    render(
      <MemoryRouter>
        <PlatformResourceSummary platform={platform} />
      </MemoryRouter>,
    );

    expect(screen.getByText('75 B / 100 B')).toBeVisible();
    expect(screen.getByText('75 B / 100 B').closest('[title]')).toHaveAttribute(
      'title',
      'Docker storage filesystem, 75.00% used',
    );
  });

  it('renders workload status counts with the platform list icon colors', () => {
    const platform = {
      ...platformDefaults(),
      id: 'platform-1',
      status: PlatformStatus.Online,
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
        healthy: 3,
        degraded: 0,
        failed: 0,
        stopped: 1,
        paused: 1,
        inProgress: 0,
        unknown: 0,
      },
      imageCount: 12,
      volumeCount: 8,
      platformDescriptor: {
        $type: 'Docker',
        daemonId: 'daemon-id',
        containerCount: 6,
        containersRunning: 4,
        containersStopped: 1,
        containersPaused: 1,
        imageUsedBytes: 20 * 1024 ** 3,
        volumeUsedBytes: 3 * 1024 ** 3,
      },
      stats: [],
    } as PlatformView;

    render(
      <MemoryRouter>
        <PlatformResourceSummary
          platform={platform}
          backupSummary={{
            platformId: platform.id,
            policyCount: 3,
            enabledPolicyCount: 3,
            dockerVolumePolicyCount: 1,
            stackPolicyCount: 1,
            deploymentPolicyCount: 1,
            swarmServicePolicyCount: 0,
            attentionPolicyCount: 0,
            lastRunStatus: null,
            lastRunAt: null,
          }}
        />
      </MemoryRouter>,
    );

    expect(screen.getByLabelText('Running: 4')).toBeVisible();
    expect(screen.getByLabelText('Healthy: 2')).toBeVisible();
    expect(screen.getByLabelText('Healthy: 3')).toBeVisible();
    expect(screen.getAllByLabelText('Paused: 1')).toHaveLength(2);
    expect(screen.getByRole('link', { name: /Containers/ }).querySelector('svg')).toHaveClass('text-emerald-500');
    expect(screen.getByRole('link', { name: /Deployments/ }).querySelector('svg')).toHaveClass('text-sky-500');
    expect(screen.getByRole('link', { name: /Backups/ })).toHaveAttribute('href', '/backup-policies');
    expect(screen.getByLabelText('Volume policies: 1')).toBeVisible();
    expect(screen.getByLabelText('Stack policies: 1')).toBeVisible();
    expect(screen.getByLabelText('Deployment policies: 1')).toBeVisible();
    expect(screen.getByRole('link', { name: /Stacks/ }).querySelector('svg')).toHaveClass('text-violet-500');
    expect(screen.getByRole('link', { name: /Images/ }).querySelector('svg')).toHaveClass('text-orange-500');
    expect(screen.getByRole('link', { name: /Volumes/ }).querySelector('svg')).toHaveClass('text-amber-500');
    expect(screen.getByRole('link', { name: /Networks/ }).querySelector('svg')).toHaveClass('text-cyan-500');
    expect(screen.getByText('20 GB')).toHaveAttribute('title', 'Disk space used by Docker image layers');
    expect(screen.getByText('3 GB')).toHaveAttribute('title', 'Disk space used by Docker local volumes');
  });

  it('leaves Swarm-specific backup and network cards for the upper summary', () => {
    const platform = {
      ...platformDefaults(),
      id: 'swarm-1',
      type: PlatformType.DockerSwarm,
      status: PlatformStatus.Online,
      platformDescriptor: {
        $type: 'Docker',
        daemonId: 'daemon-id',
        containerCount: 1,
        containersRunning: 1,
        containersStopped: 0,
        containersPaused: 0,
      },
      deploymentStatusCounts: platformDefaults().deploymentStatusCounts,
      stackStatusCounts: platformDefaults().stackStatusCounts,
      stats: [],
    } as PlatformView;

    render(
      <MemoryRouter>
        <PlatformResourceSummary platform={platform} />
      </MemoryRouter>,
    );

    expect(screen.queryByRole('link', { name: /Backups/ })).not.toBeInTheDocument();
    expect(screen.queryByRole('link', { name: /Networks/ })).not.toBeInTheDocument();
  });

  it('does not report valid current metrics as unavailable while history is empty', () => {
    const platform = {
      ...platformDefaults(),
      status: PlatformStatus.Online,
      stats: [
        {
          created: 2,
          rxBytes: 0,
          txBytes: 0,
          cpuUsage: 0,
          memoryUsage: 0,
          diskUsedBytes: 75,
          diskTotalBytes: 100,
          diskUsage: 75,
        },
      ],
    } as PlatformView;

    render(<DiskUsageChart platform={platform} stats={[]} isLoading={false} windowHours={24} />);

    expect(screen.getByText('No historical disk readings yet.')).toBeVisible();
    expect(screen.queryByText('Disk metrics unavailable')).not.toBeInTheDocument();
  });
});
