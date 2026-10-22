import { render, screen } from '@testing-library/react';
import { ContainerStateStatus, type ContainerRuntimeView, type ContainerStatView } from '@/api/generated/api.types';
import { useRead } from '@/lib/hooks';
import { ContainerStats, DeploymentStats } from './container-stats';

vi.mock('@/lib/hooks', () => ({ useRead: vi.fn() }));
vi.mock('./stats/memory-usage', () => ({
  default: ({ stats }: { stats: ContainerStatView[] }) => <div data-testid="samples">{JSON.stringify(stats)}</div>,
}));
vi.mock('./stats/cpu-usage', () => ({ default: () => null }));
vi.mock('./stats/network-usage', () => ({ default: () => null }));

const stat = (created: number, cpuUsage: number) => ({ created, cpuUsage }) as ContainerStatView;
const container = (id: string, sample: ContainerStatView) =>
  ({ id, state: ContainerStateStatus.Running, containerStat: sample }) as ContainerRuntimeView;
const readings = () => JSON.parse(screen.getByTestId('samples').textContent!) as ContainerStatView[];

it('keeps deployment readings through an empty replacement interval and refreshes settled history', () => {
  const now = Math.floor(Date.now() / 1000);
  const refetch = vi.fn();
  const result = { data: { data: { stats: [stat(now - 100, 5)] } }, isLoading: false, isFetching: false, refetch };
  vi.mocked(useRead).mockReturnValue(result as unknown as ReturnType<typeof useRead>);
  const { rerender } = render(
    <DeploymentStats deploymentId="deployment" resource={container('old', stat(now - 90, 10))} />,
  );
  rerender(<DeploymentStats deploymentId="deployment" resource={container('old', stat(now - 60, 20))} />);
  expect(readings().map((s) => s.cpuUsage)).toEqual([5, 20]);
  expect(refetch).not.toHaveBeenCalled();

  vi.mocked(useRead).mockReturnValue({ ...result, isFetching: true } as unknown as ReturnType<typeof useRead>);
  rerender(<DeploymentStats deploymentId="deployment" resource={undefined} />);
  expect(readings().map((s) => s.cpuUsage)).toEqual([5, 20]);
  expect(refetch).not.toHaveBeenCalled();
  rerender(<DeploymentStats deploymentId="deployment" resource={container('new', stat(now - 10, 30))} />);
  expect(readings().map((s) => s.cpuUsage)).toEqual([5, 20, 30]);
  vi.mocked(useRead).mockReturnValue(result as unknown as ReturnType<typeof useRead>);
  rerender(<DeploymentStats deploymentId="deployment" resource={container('new', stat(now - 10, 30))} />);
  expect(refetch).toHaveBeenCalledTimes(1);
});

it('keeps individual container live history scoped to its instance', () => {
  const now = Math.floor(Date.now() / 1000);
  vi.mocked(useRead).mockReturnValue({
    data: { data: { stats: [stat(now - 100, 5)] } },
    isLoading: false,
  } as unknown as ReturnType<typeof useRead>);
  const { rerender } = render(<ContainerStats resource={container('old', stat(now - 90, 10))} />);
  rerender(<ContainerStats resource={container('old', stat(now - 60, 20))} />);
  expect(readings().map((s) => s.cpuUsage)).toEqual([5, 20]);
  rerender(<ContainerStats resource={container('new', stat(now - 10, 30))} />);
  expect(readings().map((s) => s.cpuUsage)).toEqual([5]);
});
