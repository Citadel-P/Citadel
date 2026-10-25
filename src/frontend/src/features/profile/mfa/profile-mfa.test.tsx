import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { ProfileMfaCommand, ProfileMfaStatusBadge } from './profile-mfa';

const statusUrl = 'http://localhost/api/v1/profile/mfa';

it('updates the shared badge when the MFA status query is refreshed', async () => {
  let enabled = false;
  server.use(
    http.get(statusUrl, () =>
      HttpResponse.json({ enabled, canDisable: true, policy: 'Optional', remainingRecoveryCodes: 0 }),
    ),
  );

  const { queryClient } = renderCitadel(<ProfileMfaStatusBadge canUseLocalPassword />);
  const badge = await screen.findByRole('status', { name: 'Two-factor authentication: Disabled' });
  expect(badge.querySelector('[data-slot="badge"]')).toHaveClass('text-muted-foreground');

  enabled = true;
  await act(async () => {
    await queryClient.invalidateQueries({ queryKey: ['getProfileMfaStatus'] });
  });

  await waitFor(() => expect(badge).toHaveAccessibleName('Two-factor authentication: Enabled'));
  expect(badge.querySelector('[data-slot="badge"]')).toHaveClass('text-green-700');
});

it('shows loading until the status is known', async () => {
  let release: (() => void) | undefined;
  server.use(
    http.get(statusUrl, async () => {
      await new Promise<void>((resolve) => {
        release = resolve;
      });
      return HttpResponse.json({ enabled: true, canDisable: true, policy: 'Optional', remainingRecoveryCodes: 5 });
    }),
  );

  renderCitadel(<ProfileMfaStatusBadge canUseLocalPassword />);
  expect(screen.getByRole('status')).toHaveTextContent('Loading…');
  expect(screen.getByRole('status')).not.toHaveTextContent('Disabled');
  await waitFor(() => expect(release).toBeTypeOf('function'));
  release!();
  expect(await screen.findByRole('status', { name: 'Two-factor authentication: Enabled' })).toBeVisible();
});

it('does not label a failed status read as disabled', async () => {
  server.use(http.get(statusUrl, () => HttpResponse.json({ status: 500, title: 'Unavailable' }, { status: 500 })));
  renderCitadel(<ProfileMfaStatusBadge canUseLocalPassword />);
  const badge = await screen.findByRole('status', { name: 'Two-factor authentication: Unavailable' });
  expect(badge).not.toHaveTextContent('Disabled');
});

it('does not request local-password MFA status for an ineligible account', async () => {
  const request = vi.fn(() => HttpResponse.json({ enabled: false }));
  server.use(http.get(statusUrl, request));
  renderCitadel(<ProfileMfaStatusBadge canUseLocalPassword={false} />);

  expect(screen.getByRole('status')).toHaveTextContent('Unavailable');
  expect(screen.getByRole('status').querySelector('[data-slot="badge"]')).toHaveClass('text-muted-foreground');
  await act(async () => {});
  expect(request).not.toHaveBeenCalled();
});

it('shows an unavailable settings state and retries without offering enrollment on a failed read', async () => {
  let unavailable = true;
  server.use(
    http.get(statusUrl, () =>
      unavailable
        ? HttpResponse.json({ status: 500, title: 'Unavailable' }, { status: 500 })
        : HttpResponse.json({ enabled: false, canDisable: true, policy: 'Optional', remainingRecoveryCodes: 0 }),
    ),
  );
  const { user } = renderCitadel(<ProfileMfaCommand canUseLocalPassword />);
  await user.click(screen.getByRole('button', { name: 'Manage two-factor authentication' }));
  expect(await screen.findByText('Unable to load your two-factor authentication settings.')).toBeVisible();
  expect(screen.queryByRole('button', { name: 'Enable' })).not.toBeInTheDocument();

  unavailable = false;
  await user.click(screen.getByRole('button', { name: 'Retry' }));
  expect(await screen.findByRole('button', { name: 'Enable' })).toBeVisible();
  expect(screen.getByText('Disabled')).toBeVisible();
});
