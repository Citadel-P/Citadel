import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ContainerStateStatus, type ContainerRuntimeView, type ContainerStatView } from '@/api/generated/api.types';
import { useRead } from '@/lib/hooks';
import { buildServiceSeries, StackStats } from './stack-stats';

vi.mock('@/lib/hooks', () => ({ useRead: vi.fn() }));
vi.mock('@/components/ui/chart', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/components/ui/chart')>()),
  ChartContainer: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));
vi.mock('recharts', async (importOriginal) => ({
  ...(await importOriginal<typeof import('recharts')>()),
  AreaChart: ({ data }: { data: unknown[] }) => <div data-testid="chart" data-samples={JSON.stringify(data)} />,
}));
const sample = (created: number, cpuUsage: number): ContainerStatView => ({
  containerId: 'projection',
  created,
  cpuUsage,
  memoryActive: 100,
  memoryCache: 2,
  memoryLimit: 200,
  rxBytes: 3,
  txBytes: 4,
});
const container = (id: string) => ({ id, name: id, state: ContainerStateStatus.Running }) as ContainerRuntimeView;

it('keeps service history after containers disappear and sums live replicas once', () => {
  const history = [{ serviceName: 'web', containerIds: ['new', 'replica'], stats: [sample(101, 5), sample(111, 30)] }];
  const series = buildServiceSeries(
    history,
    [container('new'), container('replica')],
    {
      new: [sample(121, 10), sample(129, 20)],
      replica: [sample(125, 50)],
    },
    10,
  );
  expect(series).toHaveLength(1);
  expect(series[0].name).toBe('web');
  expect(series[0].stats.map((s) => [s.created, s.cpuUsage])).toEqual([
    [100, 5],
    [110, 30],
    [120, 65],
  ]);
  expect(buildServiceSeries(history, [], {}, 10)[0].stats).toEqual(history[0].stats);
  expect(buildServiceSeries([], [container('unmapped')], { unmapped: [sample(111, 999)] }, 10)).toEqual([]);
});

it('shows retained services with no current containers and refreshes membership on replacement', async () => {
  const now = Math.floor(Date.now() / 1000);
  const refetch = vi.fn();
  vi.mocked(useRead).mockReturnValue({
    data: {
      data: {
        services: [
          { serviceName: 'web', containerIds: [], stats: [sample(now - 60, 10)] },
          { serviceName: 'db', containerIds: [], stats: [sample(now - 60, 20)] },
        ],
      },
    },
    isLoading: false,
    refetch,
  } as unknown as ReturnType<typeof useRead>);
  const { rerender } = render(<StackStats stackId="stack" containers={[]} />);
  expect(screen.getAllByText('All services')).toHaveLength(3);
  expect(screen.getAllByTestId('chart')).toHaveLength(3);
  expect(refetch).not.toHaveBeenCalled();
  const user = userEvent.setup();
  await user.click(screen.getAllByRole('combobox')[1]);
  expect(screen.getByRole('option', { name: 'web' })).toBeVisible();
  expect(screen.getByRole('option', { name: 'db' })).toBeVisible();
  await user.keyboard('{Escape}');
  rerender(<StackStats stackId="stack" containers={[container('replacement')]} />);
  expect(refetch).toHaveBeenCalledTimes(1);
  rerender(
    <StackStats stackId="stack" containers={[{ ...container('replacement'), containerStat: sample(now, 10) }]} />,
  );
  expect(refetch).toHaveBeenCalledTimes(1);
});

it('preserves saved totals when live replicas report partially or a closed bucket is replayed', () => {
  const history = [{ serviceName: 'web', containerIds: ['a', 'b'], stats: [sample(101, 80), sample(111, 100)] }];
  const series = buildServiceSeries(
    history,
    [container('a'), container('b')],
    {
      a: [sample(101, 10), sample(111, 20), sample(121, 30)],
      b: [sample(101, 10), sample(121, 40)],
    },
    10,
  );
  expect(series[0].stats.map((s) => [s.created, s.cpuUsage])).toEqual([
    [100, 80],
    [110, 100],
    [120, 70],
  ]);
  const stopped = { ...container('b'), state: ContainerStateStatus.Exited };
  expect(
    buildServiceSeries(history, [container('a'), stopped], { a: [sample(121, 30)] }, 10)[0].stats.at(-1)?.cpuUsage,
  ).toBe(30);
});

it('can select a service named all without selecting every service', async () => {
  const now = Math.floor(Date.now() / 1000);
  vi.mocked(useRead).mockReturnValue({
    data: {
      data: {
        services: [
          { serviceName: 'all', containerIds: [], stats: [sample(now - 60, 10)] },
          { serviceName: 'web', containerIds: [], stats: [sample(now - 60, 20)] },
        ],
      },
    },
    isLoading: false,
    refetch: vi.fn(),
  } as unknown as ReturnType<typeof useRead>);
  render(<StackStats stackId="stack" containers={[]} />);
  const user = userEvent.setup();
  // Time range, memory service, CPU service, network service.
  await user.click(screen.getAllByRole('combobox')[2]);
  await user.click(screen.getByRole('option', { name: 'all' }));
  expect(screen.getAllByRole('combobox')[2]).toHaveTextContent('all');
  const cpu = JSON.parse(screen.getAllByTestId('chart')[1].getAttribute('data-samples')!);
  expect(cpu.at(-1).cpuUsage).toBe(10);
});

it('waits for an older history request before refreshing changed membership', () => {
  const refetch = vi.fn();
  const result = { data: { data: { services: [] } }, isLoading: true, isFetching: true, refetch };
  vi.mocked(useRead).mockReturnValue(result as unknown as ReturnType<typeof useRead>);
  const { rerender } = render(<StackStats stackId="stack" containers={[]} />);
  rerender(<StackStats stackId="stack" containers={[container('replacement')]} />);
  expect(refetch).not.toHaveBeenCalled();
  vi.mocked(useRead).mockReturnValue({ ...result, isLoading: false, isFetching: false } as unknown as ReturnType<
    typeof useRead
  >);
  rerender(<StackStats stackId="stack" containers={[container('replacement')]} />);
  expect(refetch).toHaveBeenCalledTimes(1);
});

it('keeps current stack usage separate from the chart averages', () => {
  const now = Math.floor(Date.now() / 1000);
  vi.mocked(useRead).mockReturnValue({
    data: { data: { services: [{ serviceName: 'web', containerIds: ['a'], stats: [sample(now - 60, 1)] }] } },
    isLoading: false,
    refetch: vi.fn(),
  } as unknown as ReturnType<typeof useRead>);
  render(<StackStats stackId="stack" containers={[{ ...container('a'), containerStat: sample(now, 42) }]} />);
  expect(screen.getByText('42.00%')).toBeVisible();
});
