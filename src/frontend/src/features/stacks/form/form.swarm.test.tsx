import {
  PlatformType,
  StackDriftMode,
  SwarmStackCompatibilitySeverity,
  StackSource,
  StackUpdateBehavior,
} from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { StackForm } from './form';

const swarmPlatformId = '019f0000-0000-7000-8000-000000000010';
const standalonePlatformId = '019f0000-0000-7000-8000-000000000011';
const preflight = vi.fn().mockResolvedValue({ data: { isCompatible: true, issues: [] } });
const license = vi.hoisted(() => ({ enabled: true }));

const duplicateDraft = {
  draft: {
    name: 'swarm-copy',
    platformId: swarmPlatformId,
    description: null,
    stackSource: StackSource.WebEditor,
    spec: {
      $type: 'WebEditor',
      composeFile: 'services:\n  web:\n    image: nginx',
      updateBehavior: StackUpdateBehavior.Disabled,
      destroyBeforeDeploy: false,
      preDeploy: null,
      postDeploy: null,
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
  warnings: [],
};

vi.mock('@/lib/hooks', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/lib/hooks')>();
  return {
    ...actual,
    useMutate: (key: string) => ({
      mutateAsync: key === 'preflightSwarmStack' ? preflight : vi.fn(),
      isPending: false,
    }),
    useSaveResource: () => ({ save: vi.fn(), isPending: false }),
    useRead: (key: string) => {
      switch (key) {
        case 'listPlatforms':
          return {
            data: {
              data: {
                platforms: [
                  { id: swarmPlatformId, name: 'Production Swarm', type: PlatformType.DockerSwarm },
                  { id: standalonePlatformId, name: 'Local Docker', type: PlatformType.Docker },
                ],
              },
            },
          };
        case 'getStackDuplicateDraft':
          return { data: { data: duplicateDraft }, isFetching: false };
        case 'listBuildProjects':
          return { data: { data: { projects: [] } }, isFetching: false };
        case 'lookup':
          return { data: { data: [] } };
        default:
          return { data: undefined, isFetching: false };
      }
    },
  };
});

vi.mock('@/features/license/use-license-entitlements', () => ({
  useLicenseEntitlements: () => ({ hasCapability: () => license.enabled }),
}));

vi.mock('@/components/custom/common', () => ({
  ResourceSelectorField: ({ items = [] }: { items?: Array<{ id: string; type: PlatformType }> }) => (
    <span data-testid="platform-options">{items.map((item) => item.id).join(',')}</span>
  ),
}));

vi.mock('@/features/tags/components', () => ({ ResourceTagSelector: () => null }));
vi.mock('@/lib/monaco', () => ({
  MonacoEditor: ({ filename }: { filename?: string }) => <span data-testid="compose-editor-filename">{filename}</span>,
  MonacoToArrayEditor: () => null,
}));
vi.mock('monaco-editor', () => ({ MarkerSeverity: { Error: 8, Warning: 4 } }));

vi.mock('@/components/custom/form-builder', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/components/custom/form-builder')>();

  const findField = (schema: any, key: string) => {
    for (const section of Object.values(schema) as any[]) {
      for (const item of section.items ?? []) {
        if (item.kind === 'field' && item.field.key === key) return item.field;
        if (item.kind === 'group') {
          for (const child of item.items ?? []) {
            if (child.kind === 'field' && child.field.key === key) return child.field;
          }
        }
      }
    }
    return undefined;
  };

  return {
    ...actual,
    FormShell: ({
      schema,
      confirmSave,
      setUpdate,
    }: {
      schema: any;
      confirmSave?: (payload: any) => Promise<boolean>;
      setUpdate: (value: any) => void;
    }) => {
      const platform = findField(schema, 'platformId');
      const driftControl = findField(schema, 'driftPolicy.mode')?.render(undefined, vi.fn());
      return (
        <div>
          <button onClick={() => setUpdate({ platformId: standalonePlatformId, stackSource: StackSource.WebEditor })}>
            Choose Standalone
          </button>
          {platform?.render(swarmPlatformId, vi.fn())}
          {findField(schema, 'spec.composeFile')?.render(duplicateDraft.draft.spec.composeFile, vi.fn())}
          <span>{findField(schema, 'driftPolicy.mode') ? 'Drift shown' : 'Drift hidden'}</span>
          {driftControl && (
            <span
              data-testid="drift-options"
              data-default={driftControl.props.value}
              data-detect-disabled={!!driftControl.props.collection.DetectOnly.disabled}
              data-detect-license={driftControl.props.collection.DetectOnly.requiredLicense ?? ''}
              data-auto-disabled={!!driftControl.props.collection.AutoFix.disabled}
              data-auto-license={driftControl.props.collection.AutoFix.requiredLicense ?? ''}
            />
          )}
          <span>{findField(schema, 'spec.preDeploy.path') ? 'Pre-deploy shown' : 'Pre-deploy hidden'}</span>
          <span>{findField(schema, 'spec.destroyBeforeDeploy') ? 'Destroy shown' : 'Destroy hidden'}</span>
          <button
            type="button"
            onClick={() => {
              const { $type: _type, ...specWithoutDiscriminator } = duplicateDraft.draft.spec;
              void confirmSave?.({ ...duplicateDraft.draft, spec: specWithoutDiscriminator });
            }}>
            Validate Swarm Stack
          </button>
        </div>
      );
    },
  };
});

describe('Swarm Stack form', () => {
  beforeEach(() => {
    preflight.mockClear();
    license.enabled = true;
  });

  it('defaults new Standalone stacks to free detection while keeping AutoFix licensed', async () => {
    license.enabled = false;
    const { user } = renderCitadel(<StackForm mode="add" />, { route: '/stacks/add' });
    await user.click(screen.getByRole('button', { name: 'Choose Standalone' }));
    const options = await screen.findByTestId('drift-options');
    expect(options).toHaveAttribute('data-default', StackDriftMode.DetectOnly);
    expect(options).toHaveAttribute('data-detect-disabled', 'false');
    expect(options).toHaveAttribute('data-detect-license', '');
    expect(options).toHaveAttribute('data-auto-disabled', 'true');
    expect(options).toHaveAttribute('data-auto-license', 'Team');
  });

  it('adapts Standalone-only settings and runs compatibility preflight before save', async () => {
    preflight.mockResolvedValue({ data: { isCompatible: true, issues: [] } });
    const user = userEvent.setup();
    renderCitadel(<StackForm mode="add" />, { route: '/stacks/add?duplicateFrom=source-stack' });

    expect(await screen.findByText('Docker Swarm Stack')).toBeInTheDocument();
    expect(screen.getByText('Drift hidden')).toBeInTheDocument();
    expect(screen.getByText('Pre-deploy hidden')).toBeInTheDocument();
    expect(screen.getByText('Destroy hidden')).toBeInTheDocument();
    expect(screen.getByTestId('platform-options')).toHaveTextContent(swarmPlatformId);
    expect(screen.getByTestId('platform-options')).not.toHaveTextContent(standalonePlatformId);
    expect(screen.getByTestId('compose-editor-filename')).toHaveTextContent('swarm-compose.yaml');

    await user.click(screen.getByRole('button', { name: 'Validate Swarm Stack' }));

    await waitFor(() => expect(preflight).toHaveBeenCalledTimes(1));
    expect(preflight).toHaveBeenCalledWith(
      expect.objectContaining({
        data: expect.objectContaining({
          spec: expect.objectContaining({ $type: 'WebEditor' }),
        }),
      }),
    );
    expect(await screen.findByText('Swarm compatible')).toBeInTheDocument();
  });

  it('keeps portability warnings visible until the user saves again', async () => {
    preflight.mockResolvedValue({
      data: {
        isCompatible: true,
        issues: [
          {
            severity: SwarmStackCompatibilitySeverity.Warning,
            code: 'mount.bind_portability',
            message: 'Bind mounts require the same host path on every eligible Swarm node.',
            fieldPath: 'services.api.volumes[0]',
          },
        ],
      },
    });
    const user = userEvent.setup();
    renderCitadel(<StackForm mode="add" />, { route: '/stacks/add?duplicateFrom=source-stack' });

    const save = await screen.findByRole('button', { name: 'Validate Swarm Stack' });
    await user.click(save);

    expect(await screen.findByText('Portability warning')).toBeInTheDocument();
    expect(screen.getByText(/Bind mounts require/)).toBeInTheDocument();
    expect(preflight).toHaveBeenCalledTimes(1);

    await user.click(save);
    expect(preflight).toHaveBeenCalledTimes(1);
  });
});
