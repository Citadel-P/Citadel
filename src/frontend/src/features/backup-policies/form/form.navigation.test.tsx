import { screen } from '@testing-library/react';
import { Route, Routes } from 'react-router';
import { renderCitadel } from '@/test/render-citadel';
import { BackupPolicyView } from '@/api/generated/api.types';
import { BackupPolicyForm } from './form';
import { BackToResource } from './back-to-resource';

const { create, captured } = vi.hoisted(() => ({
  create: vi.fn(async () => ({ data: { id: 'new-policy' } })),
  captured: vi.fn(),
}));
vi.mock('@/features/license/use-license-entitlements', () => ({
  useLicenseEntitlements: () => ({ hasCapability: () => true }),
}));
vi.mock('@/lib/hooks', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/hooks')>()),
  useRead: () => ({ data: undefined, isLoading: false }),
  useMutate: () => ({ mutateAsync: create }),
}));
vi.mock('@/components/custom/form-builder', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/components/custom/form-builder')>()),
  FormShell: (props: { original: object; onSave: (value: object) => Promise<unknown> }) => {
    captured(props);
    return (
      <button
        onClick={() =>
          void props.onSave({ ...props.original, name: 'Daily backup', backupRepositoryId: 'repository' })
        }>
        Save
      </button>
    );
  },
}));

beforeEach(() => {
  create.mockClear();
  captured.mockClear();
});

it.each([
  ['/deployments/edit/deployment', 'Deployment'],
  ['/stacks/edit/stack', 'Stack'],
  ['/swarm-services/edit/service', 'Swarm Service'],
  ['/platforms/p/containers/container/', 'Container'],
  ['/platforms/p/volumes', 'Volumes'],
])('opens the created policy and preserves the return link to %s', async (returnTo, label) => {
  const { user } = renderCitadel(
    <Routes>
      <Route path="/backup-policies/add" element={<BackupPolicyForm mode="add" />} />
      <Route
        path="/backup-policies/edit/new-policy"
        element={
          <>
            <BackToResource />
            <p>Created policy</p>
          </>
        }
      />
      <Route path={returnTo} element={<p>Resource overview</p>} />
    </Routes>,
    {
      route: `/backup-policies/add?sourceType=DockerVolume&platformId=p&volumeName=uploads&dockerNodeId=node-a&${new URLSearchParams({ returnTo })}`,
    },
  );
  expect(captured.mock.lastCall?.[0].original.source).toMatchObject({
    $type: 'DockerVolume',
    platformId: 'p',
    volumeName: 'uploads',
    dockerNodeId: 'node-a',
  });
  expect(captured.mock.lastCall?.[0].draftKey).toContain('new:["DockerVolume","p","node-a","uploads"]');
  await user.click(screen.getByRole('button', { name: 'Save' }));
  expect(create).toHaveBeenCalledWith({
    data: expect.objectContaining({
      source: expect.objectContaining({ dockerNodeId: 'node-a', volumeName: 'uploads' }),
    }),
  });
  expect(await screen.findByText('Created policy')).toBeVisible();
  expect(screen.queryByText('Resource overview')).not.toBeInTheDocument();
  const backLink = screen.getByRole('link', { name: `Back to ${label}` });
  expect(backLink).toHaveAttribute('href', returnTo);
  await user.click(backLink);
  expect(await screen.findByText('Resource overview')).toBeVisible();
});

it('ignores source prefill when editing an existing policy', () => {
  const resource = { id: 'existing', source: { $type: 'Stack', stackId: 'original-stack' } } as BackupPolicyView;
  renderCitadel(<BackupPolicyForm mode="edit" resource={resource} />, {
    route: '/backup-policies/edit/existing?sourceType=Deployment&deploymentId=other',
  });
  expect(captured.mock.lastCall?.[0].original.source).toEqual(resource.source);
});

it('preserves the explicit bind path and exact node when saving a contextual policy', async () => {
  const { user } = renderCitadel(
    <Routes>
      <Route path="/backup-policies/add" element={<BackupPolicyForm mode="add" />} />
      <Route
        path="/backup-policies/edit/new-policy"
        element={
          <>
            <BackToResource />
            <p>Created policy</p>
          </>
        }
      />
    </Routes>,
    {
      route: '/backup-policies/add?sourceType=BindMount&platformId=p&hostPath=%2Fsrv%2Fapp%20data&dockerNodeId=node-a',
    },
  );
  expect(captured.mock.lastCall?.[0].draftKey).toContain('BindMount');
  await user.click(screen.getByRole('button', { name: 'Save' }));
  expect(create).toHaveBeenCalledWith({
    data: expect.objectContaining({
      source: { $type: 'BindMount', platformId: 'p', hostPath: '/srv/app data', dockerNodeId: 'node-a' },
    }),
  });
  expect(await screen.findByText('Created policy')).toBeVisible();
  expect(screen.queryByRole('link', { name: /Back to/ })).not.toBeInTheDocument();
});

it('stays on the policy when saving edits with a return link', async () => {
  const resource = { id: 'existing', source: { $type: 'Stack', stackId: 'stack' } } as BackupPolicyView;
  const { user } = renderCitadel(
    <Routes>
      <Route
        path="/backup-policies/edit/:id"
        element={
          <>
            <BackToResource />
            <BackupPolicyForm mode="edit" resource={resource} />
          </>
        }
      />
      <Route path="/stacks/edit/stack" element={<p>Resource overview</p>} />
    </Routes>,
    { route: '/backup-policies/edit/existing?returnTo=%2Fstacks%2Fedit%2Fstack' },
  );
  await user.click(screen.getByRole('button', { name: 'Save' }));
  expect(create).toHaveBeenCalledWith(expect.objectContaining({ id: 'existing' }));
  expect(screen.getByRole('button', { name: 'Save' })).toBeVisible();
  expect(screen.getByRole('link', { name: 'Back to Stack' })).toBeVisible();
  expect(screen.queryByText('Resource overview')).not.toBeInTheDocument();
});
