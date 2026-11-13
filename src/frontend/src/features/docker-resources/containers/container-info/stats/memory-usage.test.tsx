import { fireEvent, render, screen } from '@testing-library/react';
import { cloneElement, type ReactElement } from 'react';
import { ContainerStateStatus } from '@/api/generated/api.types';
import { MemoryUsageCell } from '@/components/custom/common';
import MemoryUsage from './memory-usage';
import { memoryBreakdown } from '@/lib/memory-stats';

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
  memoryActiveCache: 30 * mib,
  memoryLimit: 1024 * mib,
  cpuUsage: 0,
  rxBytes: 0,
  txBytes: 0,
};

beforeEach(() => localStorage.clear());

it('shows the same working-set usage in the stats tab and Memory column without subtracting cache again', () => {
  render(
    <>
      <MemoryUsageCell state={ContainerStateStatus.Running} stats={sample} />
      <MemoryUsage
        stats={[sample]}
        container={{ state: ContainerStateStatus.Running, containerStat: sample }}
        isLoading={false}
      />
    </>,
  );

  expect(screen.getByText('94 MB / 1 GB')).toBeVisible();
  expect(screen.getByRole('radio', { name: 'Breakdown' })).toBeChecked();
  expect(screen.getByText('64 MB')).toBeVisible();
  expect(screen.getByText('30 MB')).toBeVisible();
  fireEvent.click(screen.getByRole('radio', { name: 'Working set' }));
  expect(screen.getByText('94 MB')).toBeVisible();
  expect(screen.getByRole('button', { name: 'Memory breakdown: 94 MB / 1 GB' })).toBeVisible();
  expect(screen.getByText(/Docker working set: includes active file cache/)).toBeVisible();
});

it('does not show a stopped container’s last usage as current memory', () => {
  render(<MemoryUsageCell state={ContainerStateStatus.Exited} stats={sample} />);
  expect(screen.queryByText('94 MB / 1 GB')).not.toBeInTheDocument();
  expect(screen.getByText('0B / 0B')).toBeVisible();
});

it('persists the mode across charts and falls back to breakdown for an invalid preference', () => {
  const first = render(<MemoryUsage stats={[sample]} isLoading={false} />);
  fireEvent.click(screen.getByRole('radio', { name: 'Working set' }));
  first.unmount();
  const second = render(<MemoryUsage stats={[sample]} isLoading={false} />);
  expect(screen.getByRole('radio', { name: 'Working set' })).toBeChecked();
  second.unmount();
  localStorage.setItem('stats-memory-view', '"invalid"');
  render(<MemoryUsage stats={[sample]} isLoading={false} />);
  expect(screen.getByRole('radio', { name: 'Breakdown' })).toBeChecked();
});
it('does not invent a breakdown for old or inconsistent measurements', () => {
  expect(memoryBreakdown(sample).nonCache).toBe(64 * mib);
  expect(memoryBreakdown({ ...sample, memoryActiveCache: null }).nonCache).toBeNull();
  expect(memoryBreakdown({ ...sample, memoryActiveCache: 100 * mib }).nonCache).toBeNull();
  render(<MemoryUsage stats={[{ ...sample, memoryActiveCache: null }]} isLoading={false} />);
  expect(screen.getByRole('status')).toHaveTextContent('Memory breakdown is unavailable');
  fireEvent.click(screen.getByRole('radio', { name: 'Working set' }));
  expect(screen.queryByRole('status')).not.toBeInTheDocument();
});
