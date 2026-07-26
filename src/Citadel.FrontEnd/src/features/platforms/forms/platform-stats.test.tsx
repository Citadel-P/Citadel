import { PlatformStatus, PlatformView } from '@/api/generated/api.types';
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router';
import { DiskUsageChart, getCurrentDiskUsage, normalizePlatformStats, PlatformResourceSummary } from './platform-stats';

describe('platform disk statistics', () => {
  it('preserves unavailable disk values while retaining valid zeroes', () => {
    const stats = normalizePlatformStats([
      {
        created: 1,
        diskUsedBytes: null,
        diskTotalBytes: null,
        diskUsage: null,
      },
      {
        created: 2,
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
      status: PlatformStatus.Online,
      stats: [
        {
          created: 2,
          diskUsedBytes: null,
          diskTotalBytes: null,
          diskUsage: null,
        },
        {
          created: 1,
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
        diskUsedBytes: 75,
        diskTotalBytes: 100,
        diskUsage: 101,
      },
    ]);
    const platform = {
      status: PlatformStatus.Online,
      stats: [
        {
          created: 1,
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
      status: PlatformStatus.Online,
      stats: [
        {
          created: 2,
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
      id: 'platform-1',
      status: PlatformStatus.Online,
      stats: [
        {
          created: 2,
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

  it('does not report valid current metrics as unavailable while history is empty', () => {
    const platform = {
      status: PlatformStatus.Online,
      stats: [
        {
          created: 2,
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
