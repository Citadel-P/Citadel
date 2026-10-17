import { cloneElement, type ReactElement } from 'react';
import userEvent from '@testing-library/user-event';
import { fireEvent, render, screen, within } from '@testing-library/react';
import { ContainerHealthStatus as Status, type ContainerHealthTransitionView } from '@/api/generated/api.types';
import { ContainerHealthHistory } from './container-health-history';

vi.mock('recharts', async (importOriginal) => ({
  ...(await importOriginal<typeof import('recharts')>()),
  ResponsiveContainer: ({ children }: { children: ReactElement<{ width: number; height: number }> }) =>
    cloneElement(children, { width: 600, height: 200 }),
}));
const selectRange = async (name: string) => {
  const user = userEvent.setup();
  await user.click(screen.getByRole('combobox', { name: 'Time range' }));
  await user.click(screen.getByRole('option', { name }));
};

const hour = 3_600_000;
const now = Date.UTC(2026, 9, 16, 12, 30);
const transition = (timestamp: number, status: Status): ContainerHealthTransitionView => ({
  observedAt: timestamp / 1000,
  firstObservedAt: timestamp / 1000,
  sampleCount: 1,
  state: status === Status.NotRunning ? 'Exited' : 'Running',
  health: { status, reason: 'Recorded observation; not the current health result.', observedAt: timestamp },
  origin: 'Inspection',
});

it('shows a disabled chart without stale cached points or reconnect messages', () => {
  render(
    <ContainerHealthHistory
      entries={[transition(now, Status.Healthy)]}
      now={now}
      isConnected={false}
      isLoading
      disabledReason="No health check configured for this container."
    />,
  );
  expect(screen.getByRole('figure', { name: 'Health history' })).toHaveAttribute('aria-disabled', 'true');
  expect(screen.getByText('No health check configured for this container.')).toBeVisible();
  expect(screen.queryByRole('button', { name: /^Healthy at/ })).not.toBeInTheDocument();
  expect(screen.queryByRole('combobox')).not.toBeInTheDocument();
  expect(screen.queryByText(/Reconnecting|Loading|Connection interrupted/)).not.toBeInTheDocument();
});

it('shows brief changes separately with exact timestamps and no duplicate list', () => {
  const entries = [transition(now - 1000, Status.Unhealthy), transition(now, Status.Healthy)];
  render(<ContainerHealthHistory entries={entries} now={now} />);
  expect(screen.queryByRole('status', { name: 'Selected health observation' })).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: /^Healthy at/ }));
  const details = screen.getByRole('status', { name: 'Selected health observation' });
  expect(details.querySelector('time')).toHaveAttribute('datetime', new Date(now).toISOString());
  fireEvent.click(screen.getByRole('button', { name: /^Unhealthy at/ }));
  expect(details.querySelector('time')).toHaveAttribute('datetime', new Date(now - 1000).toISOString());
  expect(within(details).queryByText(/Recorded observation/)).not.toBeInTheDocument();
  expect(screen.queryByRole('list')).not.toBeInTheDocument();
  expect(screen.queryByText('Recorded observations')).not.toBeInTheDocument();
  expect(entries[0].health.status).toBe(Status.Unhealthy);
});

it('excludes expired and future records without inventing observations in gaps', async () => {
  render(
    <ContainerHealthHistory
      entries={[
        transition(now - 168 * hour, Status.Unhealthy),
        transition(now - 168 * hour - 1000, Status.Starting),
        transition(now + 1000, Status.Healthy),
        transition(now, Status.Healthy),
      ]}
      now={now}
    />,
  );
  await selectRange('7 days');
  expect(screen.getByText('2 recorded observations')).toBeVisible();
  expect(screen.getByRole('button', { name: /^Unhealthy at/ })).toBeVisible();
  expect(screen.queryByRole('button', { name: /^Starting at/ })).not.toBeInTheDocument();
});

it('keeps lifecycle evidence even when the health status is unchanged', async () => {
  const entries = [
    { ...transition(now, Status.NotRunning), oomKilled: true, exitCode: 137 },
    { ...transition(now - hour, Status.NotRunning), state: 'Paused' },
  ];
  render(<ContainerHealthHistory entries={entries} now={now} />);
  await selectRange('7 days');
  const points = screen.getAllByRole('button', { name: /^Not running at/ });
  fireEvent.click(points[0]);
  const details = screen.getByRole('status', { name: 'Selected health observation' });
  expect(points).toHaveLength(2);
  expect(within(details).getByText('Exit code 137')).toBeVisible();
  expect(within(details).getByText('Docker reported OOM killed')).toBeVisible();
  fireEvent.click(points[1]);
  expect(within(details).getByText('Container: Paused')).toBeVisible();
  expect(within(details).queryByText('Exit code 137')).not.toBeInTheDocument();
});

it('keeps dense observation counts and separate failures accessible without a growing list', async () => {
  const entries = Array.from({ length: 200 }, (_, index) => transition(now - index * 1000, Status.Healthy));
  entries[199] = transition(now - 24 * hour, Status.Unhealthy);
  render(<ContainerHealthHistory entries={entries} now={now} />);
  await selectRange('7 days');
  expect(screen.getAllByRole('button', { name: / at / })).toHaveLength(2);
  expect(screen.getByText('200 recorded observations')).toBeVisible();
  const first = screen.getAllByRole('button', { name: /^Healthy at/ })[0];
  fireEvent.click(first);
  expect(
    within(screen.getByRole('status', { name: 'Selected health observation' })).getByText(/199 observations/),
  ).toBeVisible();
  fireEvent.keyDown(first, { key: 'Home' });
  expect(screen.getByRole('button', { name: /^Unhealthy at/ })).toHaveFocus();
  expect(
    within(screen.getByRole('status', { name: 'Selected health observation' })).getByText('Unhealthy'),
  ).toBeVisible();
  expect(screen.queryByRole('list')).not.toBeInTheDocument();
});

it('spaces dense markers when switching to six hours and restores detail at fifteen minutes', async () => {
  const entries = Array.from({ length: 18 }, (_, index) => transition(now - index * 60000, Status.Healthy));
  entries.push(transition(now - 5 * hour, Status.Healthy));
  render(<ContainerHealthHistory entries={entries} now={now} />);
  await selectRange('6 hours');
  const points = screen.getAllByRole('button', { name: /^Healthy at/ });
  expect(points.length).toBeLessThan(entries.length);
  expect(screen.getByText('19 recorded observations')).toBeVisible();
  const positions = points.map((point) => Number(point.querySelector('circle')?.getAttribute('cx')));
  positions.slice(1).forEach((position, index) => expect(positions[index] - position).toBeGreaterThanOrEqual(14));
  fireEvent.click(points[0]);
  expect(
    within(screen.getByRole('status', { name: 'Selected health observation' })).getByText(/10 observations/),
  ).toBeVisible();
  await selectRange('15 minutes');
  expect(screen.getAllByRole('button', { name: /^Healthy at/ })).toHaveLength(16);
});

it('keeps selected evidence on its original record after a realtime insert', () => {
  const old = { ...transition(now - 1000, Status.NotRunning), exitCode: 137 };
  const { rerender } = render(<ContainerHealthHistory entries={[old]} now={now} />);
  fireEvent.click(screen.getByRole('button', { name: /^Not running at/ }));
  rerender(<ContainerHealthHistory entries={[transition(now, Status.Healthy), old]} now={now} />);
  expect(screen.getByRole('button', { name: /^Not running at/ })).toHaveAttribute('aria-pressed', 'true');
  expect(
    within(screen.getByRole('status', { name: 'Selected health observation' })).getByText('Exit code 137'),
  ).toBeVisible();
});

it('renders new realtime observations and expires old records as time advances', async () => {
  const old = transition(now - 168 * hour, Status.Starting);
  const { rerender } = render(<ContainerHealthHistory entries={[old]} now={now} />);
  await selectRange('7 days');
  rerender(<ContainerHealthHistory entries={[old, transition(now, Status.Healthy)]} now={now + 1000} />);
  expect(screen.getAllByRole('button', { name: / at / })).toHaveLength(1);
  expect(screen.getByRole('button', { name: /^Healthy at/ })).toBeVisible();
  expect(screen.queryByRole('button', { name: /^Starting at/ })).not.toBeInTheDocument();
});

it.each([{ entries: [] }, { entries: [transition(now - 169 * hour, Status.Healthy)] }])(
  'shows an empty state without retained observations',
  ({ entries }) => {
    render(<ContainerHealthHistory entries={entries} now={now} />);
    expect(screen.getByText('No observations in this time range.')).toBeVisible();
    expect(screen.queryByRole('button', { name: / at / })).not.toBeInTheDocument();
    expect(screen.getByRole('combobox', { name: 'Time range' })).toBeVisible();
  },
);

it('plots events without connecting them and lets keyboard and pointer users inspect each observation', async () => {
  render(
    <ContainerHealthHistory
      entries={[transition(now, Status.Healthy), transition(now - 1000, Status.Unhealthy)]}
      now={now}
    />,
  );
  expect(screen.queryByRole('list')).not.toBeInTheDocument();
  const healthy = screen.getByRole('button', { name: /^Healthy at/ });
  const unhealthy = screen.getByRole('button', { name: /^Unhealthy at/ });
  expect(healthy).toHaveAttribute('tabindex', '0');
  expect(unhealthy).toHaveAttribute('tabindex', '-1');
  expect(Number(healthy.querySelector('circle')?.getAttribute('cx'))).toBeGreaterThan(
    Number(unhealthy.querySelector('circle')?.getAttribute('cx')),
  );
  expect(Number(healthy.querySelector('circle')?.getAttribute('cy'))).toBeLessThan(
    Number(unhealthy.querySelector('circle')?.getAttribute('cy')),
  );
  expect(document.querySelector('.recharts-scatter-line')).not.toBeInTheDocument();
  fireEvent.keyDown(healthy, { key: 'ArrowLeft' });
  expect(unhealthy).toHaveFocus();
  expect(
    within(screen.getByRole('status', { name: 'Selected health observation' })).getByText('Unhealthy'),
  ).toBeVisible();
  fireEvent.pointerEnter(healthy, { pointerType: 'mouse' });
  expect(
    within(screen.getByRole('status', { name: 'Selected health observation' })).getByText('Unhealthy'),
  ).toBeVisible();
  fireEvent.click(healthy);
  expect(
    within(screen.getByRole('status', { name: 'Selected health observation' })).getByText('Healthy'),
  ).toBeVisible();
});

it('defaults to the last hour and lets the user select longer ranges without adding observations', async () => {
  render(
    <ContainerHealthHistory
      entries={[transition(now, Status.Healthy), transition(now - 2 * hour, Status.Unhealthy)]}
      now={now}
    />,
  );
  expect(screen.getByRole('combobox', { name: 'Time range' })).toHaveTextContent('1 hour');
  expect(screen.queryByRole('button', { name: /^Unhealthy at/ })).not.toBeInTheDocument();
  await selectRange('24 hours');
  expect(screen.getByRole('button', { name: /^Unhealthy at/ })).toBeVisible();
  expect(screen.getByText('2 recorded observations')).toBeVisible();
});

it('explains grouped observations and preserves the separate failure point', async () => {
  render(
    <ContainerHealthHistory
      entries={[
        {
          ...transition(now - 2 * hour, Status.Healthy),
          origin: 'Sample',
          sampleCount: 4,
          firstObservedAt: (now - 2 * hour - 180000) / 1000,
        },
        { ...transition(now - 2 * hour - 60000, Status.Unhealthy), origin: 'Sample' },
      ]}
      now={now}
    />,
  );
  await selectRange('24 hours');
  expect(screen.getByText('5 recorded observations')).toBeVisible();
  expect(screen.getByRole('button', { name: /^Unhealthy at/ })).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: /^Healthy at/ }));
  expect(
    within(screen.getByRole('status', { name: 'Selected health observation' })).getByText(
      /4 observations with this state/,
    ),
  ).toBeVisible();
});

it('reuses Stats presets and filters local Today and Yesterday bounds', async () => {
  const midnight = new Date(now);
  midnight.setHours(0, 0, 0, 0);
  const yesterday = midnight.getTime() - hour;
  render(
    <ContainerHealthHistory
      entries={[
        transition(now, Status.Healthy),
        transition(now - hour / 2, Status.Starting),
        transition(yesterday, Status.Unhealthy),
      ]}
      now={now}
    />,
  );
  await selectRange('15 minutes');
  expect(screen.getByText('1 recorded observation')).toBeVisible();
  await selectRange('6 hours');
  expect(screen.getByRole('button', { name: /^Starting at/ })).toBeVisible();
  await selectRange('Today');
  expect(screen.queryByRole('button', { name: /^Unhealthy at/ })).not.toBeInTheDocument();
  await selectRange('Yesterday');
  expect(screen.getByRole('button', { name: /^Unhealthy at/ })).toBeVisible();
  expect(screen.queryByRole('button', { name: /^Healthy at/ })).not.toBeInTheDocument();
});

it('dismisses selected details and returns keyboard focus to the chart', () => {
  render(<ContainerHealthHistory entries={[transition(now, Status.Healthy)]} now={now} />);
  const point = screen.getByRole('button', { name: /^Healthy at/ });
  fireEvent.click(point);
  fireEvent.click(screen.getByRole('button', { name: 'Clear observation selection' }));
  expect(screen.queryByRole('status', { name: 'Selected health observation' })).not.toBeInTheDocument();
  expect(point).toHaveFocus();
  expect(point).toHaveAttribute('aria-pressed', 'false');
});

it('keeps saved observations visible during reconnection', () => {
  render(<ContainerHealthHistory entries={[transition(now, Status.Unhealthy)]} now={now} isConnected={false} />);
  expect(screen.getByText('Reconnecting')).toBeVisible();
  expect(screen.getByRole('button', { name: /^Unhealthy at/ })).toBeVisible();
  expect(screen.getByRole('status')).toHaveTextContent('Saved observations remain available');
});
