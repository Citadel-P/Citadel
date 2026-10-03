import { ContainerStateStatus, UpdateBehavior } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { DeploymentForm } from './form';

const adoptionDraft = {
  source: {
    id: '019f0000-0000-7000-8000-000000000001',
    dockerContainerId: 'container-1',
    name: '/nginx',
    platformId: '019f0000-0000-7000-8000-000000000002',
    platformName: 'Local',
    state: ContainerStateStatus.Running,
  },
  draft: {
    name: 'nginx',
    platformId: '019f0000-0000-7000-8000-000000000002',
    description: null,
    spec: {
      image: {
        $type: 'External',
        registryId: '019f0000-0000-7000-8000-000000000003',
        imageTag: 'nginx:latest',
      },
      updateBehavior: UpdateBehavior.Disabled,
    },
    tagIds: [],
  },
  issues: [],
  previewFingerprint: 'preview-fingerprint',
  canImportSensitiveEnvironmentValues: false,
};

vi.mock('@/lib/hooks', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/lib/hooks')>();

  return {
    ...actual,
    useMutate: () => ({ mutateAsync: vi.fn(), isPending: false }),
    useSaveResource: () => ({ save: vi.fn(), isPending: false }),
    useRead: (key: string) => {
      switch (key) {
        case 'getContainerAdoptionDraft':
          return { data: { data: adoptionDraft }, isFetching: false };
        case 'listBuildProjects':
          return { data: { data: { projects: [] } }, isFetching: false };
        case 'lookup':
          return { data: { data: [] } };
        case 'getExposedPorts':
          return { data: undefined, isSuccess: false };
        default:
          return { data: undefined, isFetching: false };
      }
    },
  };
});

vi.mock('@/features/license/use-license-entitlements', () => ({
  useLicenseEntitlements: () => ({ hasCapability: () => true }),
}));

vi.mock('@/components/custom/common', () => ({
  MultiResourceSelectorField: () => null,
  ResourceSelectorField: () => null,
}));

vi.mock('@/features/tags/components', () => ({
  ResourceTagSelector: () => null,
}));

vi.mock('@/lib/monaco', () => ({
  MonacoToArrayEditor: () => null,
  MonacoToDictionaryEditor: () => null,
}));

vi.mock('@/components/custom/form-builder', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/components/custom/form-builder')>();

  return {
    ...actual,
    FormShell: ({ confirmSave }: { confirmSave?: (payload: any) => Promise<boolean> }) => (
      <button type="button" onClick={() => void confirmSave?.(adoptionDraft.draft)}>
        Submit adoption
      </button>
    ),
  };
});

describe('Deployment container adoption confirmation', () => {
  it('uses the standard confirmation dialog and requires the container name', async () => {
    const user = userEvent.setup();
    renderCitadel(<DeploymentForm mode="add" />, {
      route: '/deployments/add?adoptFrom=019f0000-0000-7000-8000-000000000001',
    });

    await user.click(screen.getByRole('button', { name: 'Submit adoption' }));

    expect(await screen.findByRole('heading', { name: 'Confirm Adopt' })).toBeInTheDocument();
    const confirmButton = screen.getByRole('button', { name: 'Adopt' });
    expect(confirmButton).toBeDisabled();

    await user.type(screen.getByRole('textbox', { name: 'Enter nginx to confirm' }), 'nginx');

    expect(confirmButton).toBeEnabled();
    await user.click(confirmButton);
    await waitFor(() => expect(screen.queryByRole('heading', { name: 'Confirm Adopt' })).not.toBeInTheDocument());
  });
});
