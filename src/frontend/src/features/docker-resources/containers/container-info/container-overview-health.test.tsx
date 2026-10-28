vi.mock('@/lib/use-profile-date-time', () => ({ useProfileDateTimeFormatter: () => String }));

import { screen, waitFor, within } from '@testing-library/react';
import { renderCitadel } from '@/test/render-citadel';
import { ContainerHealthStatus as Status, ContainerStateStatus, type ContainerView } from '@/api/generated/api.types';
import type { ContainerDetailsView } from '../hooks/useContainerInfoGroup';
import { ContainerOverviewHealth } from './container-overview-health';
import { ContainerTableHealth } from './container-health-card';

const { read } = vi.hoisted(() => ({ read: vi.fn() }));
vi.mock('@/lib/hooks', () => ({ useRead: read }));
vi.mock('../hooks/useContainerHealthHistory', () => ({
  useContainerHealthHistory: () => ({ entries: [], isConnected: true, isLoading: false }),
}));

const resource = (): ContainerDetailsView =>
  ({
    resourceId: 'persisted-id',
    id: 'docker-id',
    name: 'app',
    state: 'Running',
    capabilities: { canInspect: true },
    health: { status: Status.Healthy, observedAt: Date.now(), failingStreak: 0, reason: 'Passing' },
  }) as ContainerDetailsView;

beforeEach(() => {
  read.mockReset();
  read.mockReturnValue({ data: { data: [] }, refetch: vi.fn(), isFetching: false });
});

it('opens on hover without moving focus or requesting inspection, and closes after leaving', async () => {
  const { user } = renderCitadel(<ContainerOverviewHealth resource={resource()} />);
  const trigger = screen.getByRole('button', { name: 'Health: Healthy' });
  expect(trigger).toHaveTextContent('Healthy');
  await user.hover(trigger);
  const card = screen.getByRole('dialog', { name: 'Container health' });
  expect(trigger).not.toHaveFocus();
  await user.hover(card);
  expect(screen.getByText('Consecutive failures')).toBeVisible();
  expect(read).not.toHaveBeenCalled();
  await user.unhover(card);
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
});

it('supports keyboard entry and restores focus on Escape', async () => {
  const { user } = renderCitadel(<ContainerOverviewHealth resource={resource()} />);
  const trigger = screen.getByRole('button', { name: 'Health: Healthy' });
  await user.tab();
  expect(trigger).toHaveFocus();
  await user.tab();
  expect(screen.getByRole('dialog', { name: 'Container health' })).toHaveFocus();
  await user.tab();
  expect(screen.getByRole('dialog', { name: 'Container health' }).querySelector('time')).toHaveAttribute('title');
  expect(screen.getByRole('button', { name: 'View details' })).toHaveFocus();
  await user.keyboard('{Escape}');
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(trigger).toHaveFocus();
});

it('pins on click, toggles closed, and dismisses on outside click', async () => {
  const { user } = renderCitadel(
    <>
      <ContainerOverviewHealth resource={resource()} />
      <button>Outside</button>
    </>,
  );
  const trigger = screen.getByRole('button', { name: 'Health: Healthy' });
  await user.click(trigger);
  await user.unhover(trigger);
  expect(screen.getByRole('dialog')).toBeVisible();
  await user.click(trigger);
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  await user.click(trigger);
  await user.click(screen.getByRole('button', { name: 'Outside' }));
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
});

it.each([true, false])('loads details only on request and respects Inspect permission (%s)', async (canInspect) => {
  const container = resource();
  container.capabilities!.canInspect = canInspect;
  const { user } = renderCitadel(<ContainerOverviewHealth resource={container} />);
  const trigger = screen.getByRole('button', { name: 'Health: Healthy' });
  await user.click(trigger);
  expect(read).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button', { name: 'View details' }));
  expect(screen.getByRole('dialog', { name: 'Health details' })).toBeVisible();
  expect(screen.getByRole('dialog', { name: 'Health details' })).toHaveAttribute('data-slot', 'dialog-content');
  expect(read).toHaveBeenCalledWith(
    'inspectContainer',
    { id: 'persisted-id' },
    expect.objectContaining({ enabled: false }),
  );
  if (canInspect) {
    await user.click(screen.getByRole('button', { name: 'Probe details' }));
    expect(read).toHaveBeenLastCalledWith(
      'inspectContainer',
      { id: 'persisted-id' },
      expect.objectContaining({ enabled: true }),
    );
  }
  expect(read.mock.calls.every(([name]) => name === 'inspectContainer')).toBe(true);
  await user.click(screen.getByRole('button', { name: 'Close' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  expect(trigger).toHaveFocus();
});

it('uses the same card and lazy details in a table row, addressing the persisted container ID', async () => {
  const detail = resource();
  const container = {
    ...detail,
    id: 'persisted-row-id',
    containerId: 'docker-id',
    platformId: 'platform-id',
  } as unknown as ContainerView;
  const { user, rerender } = renderCitadel(<ContainerTableHealth container={container} />);
  const trigger = screen.getByRole('button', { name: 'Health: Healthy' });
  expect(trigger.textContent).toBe('');
  expect(trigger.querySelector('svg')).toBeInTheDocument();
  await user.hover(trigger);
  expect(screen.getByRole('dialog', { name: 'Container health' })).toBeVisible();
  expect(within(screen.getByRole('dialog', { name: 'Container health' })).getByText('Healthy')).toBeVisible();
  expect(screen.getByText('Consecutive failures')).toBeVisible();
  expect(read).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button', { name: 'View details' }));
  const dialog = screen.getByRole('dialog', { name: 'Health details' });
  expect(dialog).toHaveAttribute('data-slot', 'dialog-content');
  expect(read).toHaveBeenLastCalledWith(
    'inspectContainer',
    { id: 'persisted-row-id' },
    expect.objectContaining({ enabled: false }),
  );
  rerender(
    <ContainerTableHealth container={{ ...container, health: { ...container.health!, status: Status.Unhealthy } }} />,
  );
  expect(within(dialog).getByText('Unhealthy')).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Probe details' }));
  expect(read).toHaveBeenLastCalledWith(
    'inspectContainer',
    { id: 'persisted-row-id' },
    expect.objectContaining({ enabled: true }),
  );
  await user.keyboard('{Escape}');
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  expect(trigger).toHaveFocus();
});

it('opens existing health links as a summary without fetching probe output', () => {
  renderCitadel(<ContainerOverviewHealth resource={resource()} />, { route: '/containers/persisted-id#health' });
  expect(screen.getByRole('dialog', { name: 'Container health' })).toBeVisible();
  expect(read).not.toHaveBeenCalled();
});

it('updates status and semantic color when live evidence changes or becomes stale', async () => {
  const container = resource();
  const { user, rerender } = renderCitadel(<ContainerOverviewHealth resource={container} />);
  const trigger = screen.getByRole('button', { name: 'Health: Healthy' });
  expect(trigger).toHaveClass('text-emerald-600');
  await user.click(trigger);
  rerender(
    <ContainerOverviewHealth resource={{ ...container, health: { ...container.health!, status: Status.Unhealthy } }} />,
  );
  expect(screen.getByRole('button', { name: 'Health: Unhealthy' })).toHaveClass('text-destructive');
  rerender(<ContainerOverviewHealth resource={{ ...container, state: ContainerStateStatus.Exited }} />);
  const inactive = screen.getByRole('button', { name: 'Health: Not running' });
  expect(inactive).toHaveTextContent('—');
  expect(inactive).toHaveClass('text-muted-foreground');
  expect(screen.getByText('Health checks are not running while this container is inactive.')).toBeVisible();
  rerender(<ContainerOverviewHealth resource={{ ...container, projectionStaleSince: Date.now() }} />);
  expect(screen.getByRole('button', { name: 'Health: Unknown' })).toHaveClass('text-muted-foreground');
  expect(screen.getByText(/Current health evidence is unavailable/)).toBeVisible();
});
