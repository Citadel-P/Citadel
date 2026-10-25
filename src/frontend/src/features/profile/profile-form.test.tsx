import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { CurrentProfileAuthenticationType, type CurrentProfileView } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { ProfileForm } from './profile-form';

vi.mock('@/components/custom/appearance-customizer', () => ({ AppearanceCustomizer: () => null }));
vi.mock('@/lib/monaco', () => ({ MonacoDiff: () => null }));

const capabilities = { canRead: false, canWrite: false, canExecute: false };
const profile: CurrentProfileView = {
  id: 'profile-test',
  displayName: 'admin',
  email: 'admin@example.com',
  createdAt: '2026-10-24T12:00:00Z',
  authentication: {
    type: CurrentProfileAuthenticationType.Local,
    label: 'Local account',
    canChangePassword: false,
    canUseLocalPasswordMfa: false,
    oidcProviderId: null,
    oidcProviderName: null,
  },
  authorization: { isAdministrator: false, tags: capabilities, bindings: capabilities, alertRules: capabilities },
  directRoles: [],
  teams: [],
};

it('loads the collapsed session count once, reuses the list, and updates on refresh or invalidation', async () => {
  let reads = 0;
  server.use(
    http.get('http://localhost/api/v1/profile/sessions', () => {
      reads += 1;
      return HttpResponse.json({
        canRevokeOtherSessions: true,
        sessions: Array.from({ length: reads }, (_, index) => ({
          id: index === 0 ? 'current' : `other-${index}`,
          isCurrent: index === 0,
          displayName: index === 0 ? `Browser ${reads}` : `Other browser ${index}`,
          ipAddress: null,
          userAgent: null,
          createdAt: '2026-10-24T12:00:00Z',
          lastSeenAt: '2026-10-24T12:00:00Z',
          expiresAt: '2026-11-23T12:00:00Z',
        })),
      });
    }),
  );
  const { user, queryClient } = renderCitadel(<ProfileForm profile={profile} formatDate={String} />);
  const trigger = screen.getByRole('button', { name: /^Manage active sessions/ });
  expect(trigger).toHaveAttribute('aria-expanded', 'false');
  await waitFor(() => expect(trigger).toHaveAccessibleName('Manage active sessions: 1 session'));
  expect(reads).toBe(1);
  expect(screen.queryByText('Browser 1')).not.toBeInTheDocument();

  await user.click(trigger);
  expect(await screen.findByText('Browser 1')).toBeVisible();
  expect(screen.getByRole('button', { name: 'Sign Out Others' })).toBeDisabled();
  await user.click(trigger);
  await user.click(trigger);
  expect(await screen.findByText('Browser 1')).toBeVisible();
  await waitFor(() => expect(queryClient.isFetching()).toBe(0));
  expect(reads).toBe(1);

  await user.click(trigger);
  await act(async () => {
    await queryClient.invalidateQueries({ queryKey: ['listProfileSessions'] });
  });
  await waitFor(() => expect(trigger).toHaveAccessibleName('Manage active sessions: 2 sessions'));
  expect(reads).toBe(2);
  await user.click(trigger);
  expect(await screen.findByText('Browser 2')).toBeVisible();
  expect(reads).toBe(2);

  await user.click(screen.getByRole('button', { name: 'Refresh' }));
  expect(await screen.findByText('Browser 3')).toBeVisible();
  expect(reads).toBe(3);
  await user.click(trigger);
  expect(trigger).toHaveAttribute('aria-expanded', 'false');
  expect(trigger).toHaveAccessibleName('Manage active sessions: 3 sessions');
  await user.click(trigger);
  expect(screen.getByText('Browser 3')).toBeVisible();
  await waitFor(() => expect(queryClient.isFetching()).toBe(0));
  expect(reads).toBe(3);
});
