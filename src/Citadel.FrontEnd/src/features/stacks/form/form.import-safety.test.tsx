import { StackDriftMode, StackSource, StackUpdateBehavior } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { StackForm } from './form';

const importDraft = {
  source: {
    platformId: '019f0000-0000-7000-8000-000000000001',
    platformName: 'Local',
    projectName: 'beszel-git',
    containerIds: ['container-1'],
    containerNames: ['beszel'],
    services: [],
  },
  draft: {
    name: 'beszel-git',
    platformId: '019f0000-0000-7000-8000-000000000001',
    description: null,
    stackSource: StackSource.Git,
    spec: {
      $type: 'Git',
      gitRepoId: '019f0000-0000-7000-8000-000000000002',
      branch: 'main',
      commitSha: null,
      composePaths: ['compose.yml'],
      registryId: '019f0000-0000-7000-8000-000000000003',
      projectName: 'beszel-git',
      updateBehavior: StackUpdateBehavior.Disabled,
      webhook: null,
      destroyBeforeDeploy: false,
    },
    driftPolicy: {
      mode: StackDriftMode.Disabled,
      alertOnDrift: false,
      markDegraded: false,
      autoStartStoppedContainers: false,
      autoResumePausedContainers: false,
      removeExtraContainers: false,
    },
    tagIds: [],
  },
  issues: [],
  runtimeFingerprint: 'runtime-fingerprint',
};

vi.mock('@/lib/hooks', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/lib/hooks')>();

  return {
    ...actual,
    useMutate: (key: string) => ({
      mutateAsync:
        key === 'validateComposeProjectImportDraft'
          ? vi.fn().mockResolvedValue({
              data: {
                issues: [],
                previewFingerprint: 'preview-fingerprint',
                services: [],
              },
            })
          : vi.fn(),
      isPending: false,
    }),
    useSaveResource: () => ({ save: vi.fn(), isPending: false }),
    useRead: (key: string) => {
      switch (key) {
        case 'getComposeProjectImportDraft':
          return { data: { data: importDraft }, isFetching: false };
        case 'listBuildProjects':
          return { data: { data: { projects: [] } }, isFetching: false };
        case 'lookup':
          return { data: { data: [] } };
        case 'getGitRepositoryRefs':
          return { data: { data: { refs: [] } } };
        case 'getGitRepository':
          return {
            data: {
              data: {
                id: '019f0000-0000-7000-8000-000000000002',
                name: 'beszel',
              },
            },
            isFetching: false,
          };
        case 'discoverGitRepositoryComposeProjects':
          return {
            data: { data: { projects: [] } },
            isFetching: false,
            isSuccess: true,
            error: null,
            refetch: vi.fn(),
          };
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
  ResourceSelectorField: () => null,
}));

vi.mock('@/features/tags/components', () => ({
  ResourceTagSelector: () => null,
}));

vi.mock('@/lib/monaco', () => ({
  MonacoEditor: () => null,
  MonacoToArrayEditor: () => null,
}));

vi.mock('monaco-editor', () => ({
  MarkerSeverity: { Warning: 4 },
}));

vi.mock('@/components/custom/form-builder', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/components/custom/form-builder')>();

  const findField = (schema: any, key: string) => {
    for (const section of Object.values(schema) as any[]) {
      for (const item of section.items ?? []) {
        if (item.kind === 'field' && item.field.key === key) return item.field;
        if (item.kind === 'group') {
          for (const child of item.items ?? []) {
            if (child.kind === 'field' && child.field.key === key) return child.field;
            if (child.kind === 'row') {
              const field = child.fields?.find((candidate: any) => candidate.key === key);
              if (field) return field;
            }
          }
        }
      }
    }

    return undefined;
  };

  return {
    ...actual,
    FormShell: ({ schema, confirmSave }: { schema: any; confirmSave?: (payload: any) => Promise<boolean> }) => {
      const autoUpdate = findField(schema, 'spec.updateBehavior');
      const webhook = findField(schema, 'spec.webhook');
      const drift = findField(schema, 'driftPolicy.mode');
      const repository = findField(schema, 'spec.gitRepoId');

      return (
        <div>
          {repository?.render(importDraft.draft.spec.gitRepoId, vi.fn())}
          <button aria-label="Auto Update setting" disabled={autoUpdate?.disabled} />
          <button aria-label="Webhook setting" disabled={webhook?.disabled} />
          <button aria-label="Drift setting" disabled={drift?.disabled} />
          <button type="button" onClick={() => void confirmSave?.(importDraft.draft)}>
            Submit import
          </button>
          <span>{autoUpdate?.description}</span>
        </div>
      );
    },
  };
});

describe('Stack Compose import configuration', () => {
  it('allows reviewed update and webhook settings while keeping drift disabled', async () => {
    renderCitadel(<StackForm mode="add" />, {
      route: '/stacks/add?importPlatform=019f0000-0000-7000-8000-000000000001&importProject=beszel-git',
    });

    await waitFor(() => expect(screen.getByRole('button', { name: 'Auto Update setting' })).toBeEnabled());

    expect(screen.getByRole('button', { name: 'Webhook setting' })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'Drift setting' })).toBeDisabled();
    expect(screen.getByText(/Choose how the platform handles new stack versions/i)).toBeInTheDocument();
    expect(screen.getByText(/Drift management can be configured after the import/i)).toBeInTheDocument();
  });

  it('offers the compact repository browser action', async () => {
    renderCitadel(<StackForm mode="add" />, {
      route: '/stacks/add?importPlatform=019f0000-0000-7000-8000-000000000001&importProject=beszel-git',
    });

    expect(await screen.findByRole('button', { name: 'Browse' })).toBeEnabled();
    expect(screen.queryByText('Source files')).not.toBeInTheDocument();
  });

  it('requires the project name in the standard confirmation dialog', async () => {
    const user = userEvent.setup();
    renderCitadel(<StackForm mode="add" />, {
      route: '/stacks/add?importPlatform=019f0000-0000-7000-8000-000000000001&importProject=beszel-git',
    });

    await user.click(screen.getByRole('button', { name: 'Submit import' }));

    expect(await screen.findByRole('heading', { name: 'Confirm Import' })).toBeInTheDocument();
    const confirmButton = screen.getByRole('button', { name: 'Import' });
    expect(confirmButton).toBeDisabled();

    await user.type(screen.getByRole('textbox', { name: 'Enter beszel-git to confirm' }), 'beszel-git');

    expect(confirmButton).toBeEnabled();
    await user.click(confirmButton);
    await waitFor(() => expect(screen.queryByRole('heading', { name: 'Confirm Import' })).not.toBeInTheDocument());
  });
});
