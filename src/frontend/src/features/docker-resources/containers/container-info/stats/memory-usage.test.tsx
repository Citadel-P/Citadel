import { render, screen } from '@testing-library/react';
import { cloneElement, type ReactElement } from 'react';
import { ContainerStateStatus } from '@/api/generated/api.types';
import { MemoryUsageCell } from '@/components/custom/common';
import MemoryUsage from './memory-usage';

// Give the real chart a size in jsdom, which has no browser layout engine.
vi.mock('recharts', async (importOriginal) => ({
  ...(await importOriginal<typeof import('recharts')>()),
  ResponsiveContainer: ({ children }: { children: ReactElement<{ width: number; height: number }> }) =>
    cloneElement(children, { width: 600, height: 250 }),
}));

const mib = 1024 * 1024;
const sample = {
  created: 1_700_000_000,
  memoryActive: 94 * mib,
  memoryCache: 167 * mib,
  memoryLimit: 1024 * mib,
  cpuUsage: 0,
  rxBytes: 0,
  txBytes: 0,
};

it('shows the same non-cache usage in the stats tab and Memory column without subtracting cache again', () => {
  render(
    <>
      <MemoryUsageCell state={ContainerStateStatus.Running} stats={sample} />
      <MemoryUsage
        stats={[sample]}
        container={{ state: ContainerStateStatus.Running, containerStat: sample }}
        isLoading={false}
        windowHours={1}
      />
    </>,
  );

  expect(screen.getByText('94 MB / 1 GB')).toBeVisible();
  expect(screen.getByText('94 MB')).toBeVisible();
  expect(screen.getByText('167 MB')).toBeVisible();
  expect(screen.getByTitle('Memory usage excluding file cache / memory limit')).toBeVisible();
  expect(screen.getByText(/Memory usage excluding file cache for the past 1 hours/)).toBeVisible();
});

it('does not show a stopped container’s last usage as current memory', () => {
  render(<MemoryUsageCell state={ContainerStateStatus.Exited} stats={sample} />);
  expect(screen.queryByText('94 MB / 1 GB')).not.toBeInTheDocument();
  expect(screen.getByText('0B / 0B')).toBeVisible();
});
