import { screen, within } from '@testing-library/react';
import { BackupPolicyView } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { BackupPolicyForm } from './form';

const profile = vi.hoisted(() => ({ isAdministrator: true as boolean | undefined }));

vi.mock('@/lib/monaco', () => ({ MonacoDiff: () => null }));
vi.mock('@/features/license/use-license-entitlements', () => ({
  useLicenseEntitlements: () => ({ hasCapability: () => true }),
}));
vi.mock('@/lib/hooks', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/hooks')>()),
  useRead: (operation: string) => ({
    data:
      operation === 'getCurrentProfile' && profile.isAdministrator !== undefined
        ? { data: { authorization: { isAdministrator: profile.isAdministrator } } }
        : undefined,
    isLoading: false,
  }),
  useMutate: () => ({ mutateAsync: vi.fn() }),
  useSaveResource: () => ({ save: vi.fn(), isPending: false }),
}));

beforeEach(() => {
  profile.isAdministrator = true;
});

it.each([false, undefined])(
  'hides system recovery without confirmed administrator access (%s)',
  async (isAdministrator) => {
    profile.isAdministrator = isAdministrator;
    const { user } = renderCitadel(<BackupPolicyForm mode="add" />);
    const trigger = screen.getByRole('combobox', { name: 'Back up' });
    expect(trigger).toHaveTextContent('Deployment storage');
    expect(screen.queryByText('Offline recovery')).not.toBeInTheDocument();
    await user.click(trigger);
    expect(screen.queryByRole('group', { name: 'System recovery' })).not.toBeInTheDocument();
    expect(screen.queryByRole('option', { name: 'Citadel system' })).not.toBeInTheDocument();
    expect(screen.getByRole('option', { name: 'Docker volume' })).toBeVisible();
  },
);

it('groups backup choices and updates the accessible scope description when selecting a source', async () => {
  const { user } = renderCitadel(<BackupPolicyForm mode="add" />);
  const trigger = screen.getByRole('combobox', { name: 'Back up' });
  expect(trigger).toHaveTextContent('Citadel system');
  expect(trigger).toHaveAccessibleDescription("Back up Citadel's database and recovery keys.");

  await user.click(trigger);
  const list = screen.getByRole('listbox');
  expect(within(list).getByRole('group', { name: 'Application storage' })).toBeVisible();
  expect(within(list).getByRole('group', { name: 'Individual storage' })).toBeVisible();
  expect(within(list).getByRole('group', { name: 'System recovery' })).toBeVisible();
  expect(
    within(list)
      .getAllByRole('option')
      .map((option) => option.textContent),
  ).toEqual([
    'Deployment storage',
    'Stack storage',
    'Swarm service storage',
    'Docker volume',
    'Host file or directory',
    'Citadel system',
  ]);
  expect(within(list).queryByText(/Back up/)).not.toBeInTheDocument();
  await user.click(screen.getByRole('option', { name: 'Docker volume' }));
  expect(trigger).toHaveAccessibleDescription('Back up one named or anonymous Docker volume.');
  expect(screen.queryByText('Offline recovery')).not.toBeInTheDocument();

  await user.click(trigger);
  await user.click(screen.getByRole('option', { name: 'Host file or directory' }));
  expect(trigger).toHaveAccessibleDescription(
    'Back up a specific path on the selected host. Requires write access to the platform.',
  );
  expect(screen.getByRole('textbox', { name: 'Host path' })).toBeVisible();
});

it('keeps the source locked after a successful backup with its scope visible', () => {
  renderCitadel(
    <BackupPolicyForm
      mode="edit"
      resource={
        {
          id: 'policy',
          source: { $type: 'Deployment', deploymentId: 'deployment' },
          firstSuccessfulRunAt: '2026-10-13T12:00:00Z',
        } as BackupPolicyView
      }
    />,
  );
  const trigger = screen.getByRole('combobox', { name: 'Back up' });
  expect(trigger).toBeDisabled();
  expect(trigger).toHaveTextContent('Deployment storage');
  expect(trigger).toHaveAccessibleDescription(
    'Back up declared named volumes and supported bind mounts discovered from this deployment. Host paths require platform write access.',
  );
});

it('lets a legacy policy switch to Live without unlocking its storage source', async () => {
  const { user } = renderCitadel(
    <BackupPolicyForm
      mode="edit"
      resource={
        {
          id: 'policy',
          source: {
            $type: 'DockerVolume',
            platformId: 'platform',
            volumeName: 'data',
            consistency: 'StopAttachedContainers',
          },
          firstSuccessfulRunAt: '2026-10-13T12:00:00Z',
          capabilities: { canRead: true, canWrite: true },
        } as BackupPolicyView
      }
    />,
  );
  expect(screen.getByRole('combobox', { name: 'Back up' })).toBeDisabled();
  const consistency = screen.getByRole('combobox', { name: 'Consistency' });
  expect(consistency).toBeEnabled();
  await user.click(consistency);
  expect(screen.getByRole('option', { name: /Stop attached containers/ })).toHaveAttribute('aria-disabled', 'true');
  await user.click(screen.getByRole('option', { name: /^Live/ }));
  expect(consistency).toHaveTextContent('Live');
});
