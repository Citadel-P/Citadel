import { render, screen } from '@testing-library/react';
import type { ReactNode } from 'react';
import { useStatsChartBounds } from '@/components/custom/stats-chart-range';
import NetworkUsage from './network-usage';

vi.mock('@/components/custom/stats-chart-range', () => ({ useStatsChartBounds: vi.fn() }));
vi.mock('recharts', async (importOriginal) => ({
  ...(await importOriginal<typeof import('recharts')>()),
  ResponsiveContainer: ({ children }: { children: ReactNode }) => children,
  AreaChart: ({ data }: { data: unknown[] }) => <output data-testid="network-series">{JSON.stringify(data)}</output>,
}));

const kib = 1024;
const sample = (created: number, received?: number, sent?: number) => ({
  created,
  rxBytes: 999999999,
  txBytes: 999999999,
  rxBytesDelta: received === undefined ? null : received * kib,
  txBytesDelta: sent === undefined ? null : sent * kib,
  cpuUsage: 0,
  memoryActive: 0,
  memoryCache: 0,
  memoryLimit: 0,
});
const chart = () =>
  JSON.parse(screen.getByTestId('network-series').textContent!) as {
    created: number;
    rxBytes?: number;
    txBytes?: number;
  }[];

it('sums server traffic buckets for the selected 15-minute, 1-hour and 6-hour window', () => {
  const now = 21600;
  const stats = [sample(0), sample(18000, 30, 60), sample(20700, 20, 40), sample(now, 9, 4)];
  const renderChart = () => <NetworkUsage stats={stats} isLoading={false} />;
  vi.mocked(useStatsChartBounds).mockReturnValue({ from: now - 900, to: now });
  const { rerender } = render(renderChart());
  expect(screen.getByText('29 kB')).toBeVisible();
  expect(screen.getByText('44 kB')).toBeVisible();
  expect(chart().at(-1)).toEqual({ created: now, rxBytes: 29 * kib, txBytes: 44 * kib });

  vi.mocked(useStatsChartBounds).mockReturnValue({ from: now - 3600, to: now });
  rerender(renderChart());
  expect(screen.getByText('59 kB')).toBeVisible();
  expect(screen.getByText('104 kB')).toBeVisible();

  vi.mocked(useStatsChartBounds).mockReturnValue({ from: 0, to: now });
  rerender(renderChart());
  expect(screen.getByText('59 kB')).toBeVisible();
  expect(screen.getByText('104 kB')).toBeVisible();
});

it('uses chronological, deduplicated buckets and excludes observations outside a historical range', () => {
  vi.mocked(useStatsChartBounds).mockReturnValue({ from: 20, to: 40 });
  render(
    <NetworkUsage
      stats={[
        sample(50, 500, 900),
        sample(40, 5, 3),
        sample(10, 100, 200),
        sample(20),
        sample(30, 8, 20),
        sample(30, 10, 22),
      ]}
      isLoading={false}
    />,
  );
  expect(screen.getByText('15 kB')).toBeVisible();
  expect(screen.getByText('25 kB')).toBeVisible();
  expect(chart().map((point) => point.created)).toEqual([20, 30, 40]);
});

it('distinguishes unavailable deltas from a measured zero without falling back to lifetime counters', () => {
  vi.mocked(useStatsChartBounds).mockReturnValue(undefined);
  const { rerender } = render(<NetworkUsage stats={[]} isLoading={false} />);
  expect(screen.getAllByText('-')).toHaveLength(2);
  rerender(<NetworkUsage stats={[sample(10)]} isLoading={false} />);
  expect(screen.getAllByText('-')).toHaveLength(2);
  // One bucket can contain multiple raw samples: zero is a valid measurement.
  rerender(<NetworkUsage stats={[sample(10, 0, 0)]} isLoading={false} />);
  expect(screen.getAllByText('0 B')).toHaveLength(2);
});
