import { PlatformType, StackSource, StackUpdateBehavior } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { screen, waitFor } from '@testing-library/react';
import { Route, Routes } from 'react-router';
import { StackForm } from './form';

const { updateStack, fixture } = vi.hoisted(() => ({
  updateStack: vi.fn(),
  fixture: { config: {} as any },
}));

vi.mock('@/lib/hooks', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/hooks')>()),
  useMutate: (key: string) => ({ mutateAsync: key === 'updateStack' ? updateStack : vi.fn(), isPending: false }),
  useSaveResource: ({ onUpdate }: { onUpdate: () => Promise<unknown> }) => ({ save: onUpdate, isPending: false }),
  useRead: (key: string) => {
    const values: Record<string, unknown> = {
      getStackConfig: fixture.config,
      getStack: { ...fixture.config, status: 'Healthy' },
      listPlatforms: { platforms: [{ id: 'platform', name: 'Docker', type: PlatformType.Docker }] },
      listBuildProjects: { projects: [] },
      lookup: [],
      getGitRepository: { id: 'repository', name: 'Repository' },
      getGitRepositoryRefs: { refs: [] },
      discoverGitRepositoryComposeProjects: { projects: [{ composePaths: ['compose.yml'], envFilePaths: [] }] },
    };
    return { data: { data: values[key] }, isFetching: false, isSuccess: true, refetch: vi.fn() };
  },
}));

vi.mock('@/features/license/use-license-entitlements', () => ({
  useLicenseEntitlements: () => ({ hasCapability: () => false }),
}));
vi.mock('@/features/tags/components', () => ({ ResourceTagSelector: () => null }));
vi.mock('@/lib/monaco', () => ({ MonacoEditor: () => null, MonacoToArrayEditor: () => null, MonacoDiff: () => null }));
vi.mock('monaco-editor', () => ({ MarkerSeverity: { Error: 8, Warning: 4 } }));
vi.mock('@/components/custom/common', () => ({
  ResourceSelectorField: ({ selected, onSelect, targetType }: any) => (
    <select
      aria-label={targetType}
      value={typeof selected === 'string' ? selected : (selected?.id ?? '')}
      onChange={(event) => onSelect?.(event.target.value ? { id: event.target.value } : undefined)}>
      <option value="">None</option>
      <option value="registry">Private registry</option>
      <option value="platform">Docker</option>
      <option value="repository">Repository</option>
    </select>
  ),
}));

const saveButtons = () => screen.getAllByRole('button', { name: 'Save' });

function renderForm() {
  return renderCitadel(
    <Routes>
      <Route path="/stacks/edit/:id" element={<StackForm mode="edit" />} />
    </Routes>,
    { route: '/stacks/edit/stack' },
  );
}

describe('Stack registry is optional', () => {
  beforeEach(() => {
    updateStack.mockReset().mockResolvedValue({ data: {} });
    fixture.config = {
      id: 'stack',
      name: 'public-images',
      platformId: 'platform',
      platformType: PlatformType.Docker,
      stackSource: StackSource.Git,
      rowVersion: 1,
      spec: {
        $type: 'Git',
        gitRepoId: 'repository',
        branch: 'main',
        composePaths: ['compose.yml'],
        updateBehavior: StackUpdateBehavior.Disabled,
        destroyBeforeDeploy: false,
      },
    };
  });

  it.each([StackSource.Git, StackSource.WebEditor])(
    'saves update behavior without a registry for %s',
    async (source) => {
      if (source === StackSource.WebEditor) {
        fixture.config.stackSource = source;
        fixture.config.spec = {
          $type: 'WebEditor',
          composeFile: 'services:\n  web:\n    image: nginx:alpine',
          updateBehavior: StackUpdateBehavior.Disabled,
        };
      }
      const { user } = renderForm();
      expect(saveButtons().every((button) => button.hasAttribute('disabled'))).toBe(true);
      await user.click(screen.getByRole('combobox', { name: 'Auto Update' }));
      await user.click(screen.getByRole('option', { name: /^Notify Only/ }));
      await waitFor(() => expect(saveButtons()[0]).toBeEnabled());
      await user.click(saveButtons()[0]);
      await waitFor(() =>
        expect(updateStack).toHaveBeenCalledWith({
          id: 'stack',
          data: expect.objectContaining({
            spec: expect.objectContaining({ updateBehavior: StackUpdateBehavior.Notify }),
          }),
        }),
      );
      expect(updateStack.mock.calls[0][0].data.spec.registryId).toBeUndefined();
    },
  );

  it('can clear existing registry credentials and save', async () => {
    fixture.config.spec.registryId = 'registry';
    const { user } = renderForm();
    await user.selectOptions(screen.getByRole('combobox', { name: 'Registry' }), '');
    await waitFor(() => expect(saveButtons()[0]).toBeEnabled());
    await user.click(saveButtons()[0]);
    await waitFor(() =>
      expect(updateStack).toHaveBeenCalledWith({
        id: 'stack',
        data: expect.objectContaining({ spec: expect.objectContaining({ registryId: null }) }),
      }),
    );
  });
});
