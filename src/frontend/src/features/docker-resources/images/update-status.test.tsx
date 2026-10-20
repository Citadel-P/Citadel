import { act, cleanup, fireEvent, screen, within } from '@testing-library/react';
import { ImageUpdateStatus as Status, type ImageTagUpdate } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { ImageUpdateStatus } from './update-status';
import { afterEach, beforeEach, vi } from 'vitest';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/server';

beforeEach(() => {
  vi.useFakeTimers();
  server.use(http.get('http://localhost/api/v1/profile/preferences', () => HttpResponse.json({})));
});
afterEach(async () => {
  cleanup();
  // Hover and focus can each schedule a Radix open callback. Finish callbacks
  // while the DOM still exists, before Vitest tears down this test environment.
  await act(() => vi.runOnlyPendingTimersAsync());
  vi.useRealTimers();
});

async function openHoverCard() {
  await act(() => vi.advanceTimersByTimeAsync(150));
  return screen.getByRole('region', { name: 'Image update details' });
}

const update: ImageTagUpdate = {
  reference: 'nginx:17',
  status: Status.CheckFailed,
  isStale: false,
  reason: 'Registry access denied.',
  lastAttemptAt: '2026-10-16T09:56:00Z',
  lastSuccessAt: '2026-10-15T09:56:00Z',
  localConfigDigest: 'sha256:local',
  remoteConfigDigest: 'sha256:remote',
  remoteManifestDigest: 'sha256:manifest',
};

it('shows per-tag failure details on hover using the shared status icon', async () => {
  renderCitadel(<ImageUpdateStatus updates={[update]} />);
  const trigger = screen.getByRole('button', { name: 'Update status: Check failed' });
  expect(trigger.querySelector('svg')).toHaveClass('text-danger');
  expect(screen.queryByText(update.reason!)).not.toBeInTheDocument();
  fireEvent.pointerEnter(trigger, { pointerType: 'mouse' });
  const card = within(await openHoverCard());
  expect(card.getByText(update.reference)).toBeVisible();
  expect(card.getByText(update.reason!)).toBeVisible();
  expect(card.getByText(/Last check:/)).toBeVisible();
  expect(card.getByText(/Last successful observation:/)).toBeVisible();
  expect(card.queryByText('sha256:local')).not.toBeInTheDocument();
  fireEvent.click(card.getByRole('button', { name: 'Technical details' }));
  expect(card.getByText('sha256:local')).toBeVisible();
  expect(card.getByText('sha256:remote')).toBeVisible();
  expect(card.getByText('sha256:manifest')).toBeVisible();
});

it('uses the same plain-language update explanation in the dialog and hover card', async () => {
  const updates = [{ ...update, status: Status.UpdateAvailable }];
  const { rerender } = renderCitadel(<ImageUpdateStatus updates={updates} detail />);
  const explanation = screen.getByText(/The registry has a different image for this tag/).textContent!;
  expect(explanation).toContain('Existing workloads keep using their current image.');
  expect(screen.queryByText('sha256:remote')).not.toBeInTheDocument();
  rerender(<ImageUpdateStatus updates={updates} compact />);
  fireEvent.pointerEnter(screen.getByRole('button', { name: 'Update status: Update available' }), {
    pointerType: 'mouse',
  });
  const card = within(await openHoverCard());
  expect(card.getByText(explanation)).toBeVisible();
  expect(card.queryByText('sha256:remote')).not.toBeInTheDocument();
});

it('opens an icon-only status with keyboard focus and closes with Escape', async () => {
  renderCitadel(<ImageUpdateStatus updates={[update]} compact />);
  const trigger = screen.getByRole('button', { name: 'Update status: Check failed' });
  expect(trigger.textContent).toBe('');
  act(() => trigger.focus());
  expect(trigger).toHaveFocus();
  expect(await openHoverCard()).toBeVisible();
  fireEvent.keyDown(trigger, { key: 'Escape' });
  expect(trigger).toHaveAttribute('aria-expanded', 'false');
});

it('shows every tag and avoids a green summary for mixed or stale observations', async () => {
  const updates: ImageTagUpdate[] = [
    { reference: 'nginx:17', status: Status.UpToDate, isStale: true },
    { reference: 'nginx:latest', status: Status.NotChecked, isStale: false },
    { reference: 'nginx@sha256:pinned', status: Status.NotApplicable, reason: 'Digest pinned', isStale: false },
  ];
  renderCitadel(<ImageUpdateStatus updates={updates} />);
  const trigger = screen.getByRole('button', { name: 'Update status: Mixed status (stale)' });
  expect(trigger.querySelector('svg')).not.toHaveClass('text-success');
  fireEvent.pointerEnter(trigger, { pointerType: 'mouse' });
  act(() => trigger.focus());
  fireEvent.click(trigger);
  const card = within(await openHoverCard());
  for (const item of updates) expect(card.getByText(item.reference)).toBeVisible();
  expect(card.getByText('Stale observation')).toBeVisible();
  expect(card.getByText('Digest pinned')).toBeVisible();
});

it('keeps never-checked images discoverable with an explanatory card on touch', async () => {
  renderCitadel(<ImageUpdateStatus updates={[]} compact />);
  fireEvent.touchStart(screen.getByRole('button', { name: 'Update status: Not checked' }));
  expect(screen.getByText('No registry update check has been recorded.')).toBeVisible();
});
