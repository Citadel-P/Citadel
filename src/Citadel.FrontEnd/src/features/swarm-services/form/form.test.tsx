import {
  ManagedSwarmServiceView,
  PlatformType,
  SwarmServiceResources,
  SwarmNetworkView,
  SwarmServiceSpec,
  UpdateBehavior,
  WebhookAuthScheme,
  WebhookProvider,
} from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { screen, within } from '@testing-library/react';
import { Route, Routes } from 'react-router';
import {
  getServiceNetworkOptions,
  getSwarmResourceProfile,
  getSwarmResourcesForProfile,
  prepareSwarmServiceSpecForWrite,
  SwarmServiceForm,
} from './form';

const platformId = '00000000-0000-0000-0000-000000000200';

vi.mock('@/lib/hooks', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/lib/hooks')>();
  return {
    ...actual,
    useMutate: () => ({ mutateAsync: vi.fn() }),
    useSaveResource: () => ({ save: vi.fn(), isPending: false }),
    useRead: (resource: string) => {
      if (resource === 'listPlatforms') {
        return {
          data: {
            data: {
              platforms: [
                { id: platformId, name: 'Production Swarm', type: PlatformType.DockerSwarm, status: 'Online' },
                { id: 'standalone', name: 'Standalone', type: PlatformType.Docker, status: 'Online' },
              ],
            },
          },
          isLoading: false,
        };
      }

      if (resource === 'listRegistries') {
        return {
          data: { data: { registries: [{ id: 'registry-1', name: 'Docker Hub', registryHost: 'docker.io' }] } },
          isLoading: false,
        };
      }

      if (resource === 'listBuildProjects') {
        return { data: { data: { projects: [] } }, isLoading: false };
      }

      if (resource === 'getSwarmServiceDuplicateDraft') {
        return {
          data: {
            data: {
              draft: {
                name: 'redis-service-copy',
                platformId,
                description: 'Copied service',
                spec: {
                  image: { $type: 'External', registryId: 'registry-1', imageTag: 'redis' },
                  schedulingMode: 'Replicated',
                  replicas: 1,
                },
                tagIds: [],
                duplicateSource: {
                  resourceType: 'SwarmService',
                  resourceId: 'source-service-id',
                  resourceName: 'redis-service',
                },
              },
              warnings: [],
            },
          },
          isFetching: false,
        };
      }

      if (resource === 'getSwarmServiceAdoptionDraft') {
        return {
          data: {
            data: {
              source: {
                dockerServiceId: 'service-1',
                name: 'external-web',
                platformId,
                platformName: 'Production Swarm',
              },
              draft: {
                name: 'external-web',
                platformId,
                description: 'Adopted from Docker Swarm Service external-web.',
                spec: {
                  image: {
                    $type: 'External',
                    registryId: '00000000-0000-0000-0000-000000000000',
                    imageTag: 'redis@sha256:344e3945a0b431c8ff1eecd58c5573538126bd756f02fc7e218ddf1fc2546366',
                  },
                  updateBehavior: 'Disabled',
                  schedulingMode: 'Replicated',
                  replicas: 2,
                  command: [],
                  arguments: [],
                  environment: [],
                  labels: { team: 'platform' },
                  ports: [],
                  networkIds: [],
                  mounts: [],
                  secrets: [],
                  configs: [],
                  placementConstraints: [],
                },
                tagIds: [],
              },
              issues: [],
              previewFingerprint: 'a'.repeat(64),
            },
          },
          isFetching: false,
        };
      }

      if (resource === 'lookup') {
        return { data: { data: [{ id: 'binding-1', name: 'LOG_LEVEL' }] }, isLoading: false };
      }

      return { data: { data: { items: [] } }, isLoading: false };
    },
  };
});

vi.mock('@/features/license/use-license-entitlements', () => ({
  useLicenseEntitlements: () => ({ hasCapability: () => true }),
}));

vi.mock('@/features/tags/components', () => ({
  ResourceTagSelector: () => <div>Tag selector</div>,
}));

vi.mock('@/lib/monaco', () => ({
  MonacoToArrayEditor: ({
    helperText,
    validateItem,
  }: {
    helperText?: string;
    validateItem?: (value: string) => string | null;
  }) => (
    <div>
      <textarea aria-label={helperText ?? 'Editor'} />
      {helperText === '# LOG_LEVEL=${LOG_LEVEL}' && (
        <span data-testid="environment-diagnostic">{validateItem?.('LOG_LEVEL=${LOG_LEVELs}')}</span>
      )}
    </div>
  ),
  MonacoToDictionaryEditor: ({ helperText, value }: { helperText?: string; value?: Record<string, string> }) => (
    <textarea aria-label={helperText ?? 'Dictionary editor'} value={JSON.stringify(value ?? {})} readOnly />
  ),
  MonacoDiff: () => null,
}));

describe('SwarmServiceForm', () => {
  it('writes the polymorphic image discriminator before image properties', () => {
    const image = { registryId: 'registry-1', imageTag: 'redis', $type: 'External' } as const;
    const spec = { image } as SwarmServiceSpec;

    const result = prepareSwarmServiceSpecForWrite(spec);

    expect(Object.keys(result.image)[0]).toBe('$type');
    expect(result.image).toEqual(image);
    expect(JSON.stringify(result)).toContain('"image":{"$type":"External"');
  });

  it('restores a missing image discriminator from the selected image shape', () => {
    const spec = {
      image: { registryId: 'registry-1', imageTag: 'redis' },
    } as unknown as SwarmServiceSpec;

    expect(prepareSwarmServiceSpecForWrite(spec).image.$type).toBe('External');
  });

  it('excludes ingress from new selections but keeps an existing selection removable', () => {
    const networks = [
      { id: 'overlay', name: 'application', scope: 'Swarm', isIngress: false },
      { id: 'ingress', name: 'ingress', scope: 'Swarm', isIngress: true },
      { id: 'bridge', name: 'bridge', scope: 'Local', isIngress: false },
    ] as SwarmNetworkView[];

    expect(getServiceNetworkOptions(networks).map((network) => network.id)).toEqual(['overlay']);
    expect(getServiceNetworkOptions(networks, ['ingress']).map((network) => network.id)).toEqual([
      'overlay',
      'ingress',
    ]);
  });

  it('maps shared resource profiles to Swarm limits without inventing reservations', () => {
    const resources = getSwarmResourcesForProfile('xsmall');

    expect(resources).toEqual({
      limitNanoCpus: 250_000_000,
      limitMemoryBytes: 268_435_456,
      reservationNanoCpus: null,
      reservationMemoryBytes: null,
    });
    expect(getSwarmResourceProfile(resources)).toBe('xsmall');
    expect(getSwarmResourcesForProfile('automatic')).toBeNull();
  });

  it('shows Automatic when existing values do not match a preset', () => {
    const resources: SwarmServiceResources = {
      limitNanoCpus: 750_000_000,
      limitMemoryBytes: 768 * 1024 * 1024,
      reservationNanoCpus: 250_000_000,
    };

    expect(getSwarmResourceProfile(resources)).toBe('automatic');
  });

  it('uses the standard Citadel form sections and the platform from the route', () => {
    renderCitadel(
      <Routes>
        <Route path="/platforms/:platformId/services/add" element={<SwarmServiceForm mode="add" />} />
      </Routes>,
      { route: `/platforms/${platformId}/services/add` },
    );

    expect(screen.getByRole('textbox', { name: 'Name' })).toBeVisible();
    expect(screen.getByRole('textbox', { name: 'Description' })).toBeVisible();
    expect(screen.getByRole('combobox', { name: 'Platform' })).toHaveTextContent('Production Swarm');
    expect(screen.getAllByText('Platform and Scheduling').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Image').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Networks').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Storage').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Configuration').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Advanced').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Resources and Placement').length).toBeGreaterThan(0);
    const resourceProfile = screen.getByRole('combobox', { name: 'Resources' });
    expect(resourceProfile).toHaveTextContent('Automatic');
    expect(screen.queryByRole('spinbutton', { name: 'CPU Reservation' })).not.toBeInTheDocument();
    expect(screen.getAllByText('Health Check').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Rolling Update').length).toBeGreaterThan(0);
    expect(screen.getByRole('textbox', { name: '# KEY=value' })).toBeVisible();
    const form = screen.getByRole('main');
    const advancedHeading = within(form).getByText('Advanced');
    const webhookHeading = within(form).getByRole('heading', { name: 'Webhook' });
    expect(advancedHeading.compareDocumentPosition(webhookHeading)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
    const navigation = screen.getByRole('navigation', { name: 'Form sections' });
    expect(within(navigation).getAllByRole('link')).toHaveLength(12);
    expect(within(navigation).queryByRole('link', { name: 'Mounts' })).not.toBeInTheDocument();
    expect(within(navigation).queryByRole('link', { name: 'Auto Update' })).not.toBeInTheDocument();
    expect(within(navigation).queryByRole('link', { name: 'Placement Constraints' })).not.toBeInTheDocument();
    expect(screen.getByTestId('environment-diagnostic')).toHaveTextContent(
      'LOG_LEVELs is not defined in Service or global variables.',
    );
  }, 10_000);

  it('hydrates an external registry and image reference when editing', () => {
    const resource = {
      id: 'service-id',
      name: 'redis-service',
      platformId,
      rowVersion: 1,
      tags: [],
      spec: {
        image: {
          $type: 'External',
          registryId: 'registry-1',
          imageTag: 'redis',
        },
      },
    } as ManagedSwarmServiceView;

    renderCitadel(
      <Routes>
        <Route
          path="/platforms/:platformId/services/edit/:id"
          element={<SwarmServiceForm mode="edit" resource={resource} />}
        />
      </Routes>,
      { route: `/platforms/${platformId}/services/edit/${resource.id}` },
    );

    expect(screen.getByRole('combobox', { name: 'Registry' })).toHaveTextContent('Docker Hub');
    expect(screen.getByRole('textbox', { name: 'Image Reference' })).toHaveValue('redis');
  });

  it('disables an enabled webhook when image update behavior is disabled', async () => {
    const resource = {
      id: 'service-id',
      name: 'redis-service',
      platformId,
      rowVersion: 1,
      tags: [],
      spec: {
        image: {
          $type: 'External',
          registryId: 'registry-1',
          imageTag: 'redis',
        },
        updateBehavior: UpdateBehavior.Notify,
        webhook: {
          enabled: true,
          provider: WebhookProvider.Generic,
          authScheme: WebhookAuthScheme.BearerToken,
          secret: 'shared-secret',
        },
      },
    } as ManagedSwarmServiceView;

    const { user } = renderCitadel(
      <Routes>
        <Route
          path="/platforms/:platformId/services/edit/:id"
          element={<SwarmServiceForm mode="edit" resource={resource} />}
        />
      </Routes>,
      { route: `/platforms/${platformId}/services/edit/${resource.id}` },
    );

    expect(screen.getByText('Generic / CI')).toBeVisible();
    await user.click(screen.getByRole('combobox', { name: 'Auto Update' }));
    await user.keyboard('{Home}{Enter}');

    expect(screen.queryByText('Generic / CI')).not.toBeInTheDocument();
    const webhookSwitch = document.getElementById('swarm-service-update-webhook-enabled');
    expect(webhookSwitch).not.toBeNull();
    expect(webhookSwitch).not.toBeChecked();
  });

  it('loads a duplicate draft into the add form', async () => {
    renderCitadel(
      <Routes>
        <Route path="/platforms/:platformId/services/add" element={<SwarmServiceForm mode="add" />} />
      </Routes>,
      { route: `/platforms/${platformId}/services/add?duplicateFrom=source-service-id` },
    );

    expect(await screen.findByRole('textbox', { name: 'Name' })).toHaveValue('redis-service-copy');
    expect(screen.getByRole('textbox', { name: 'Description' })).toHaveValue('Copied service');
    expect(screen.getByRole('combobox', { name: 'Registry' })).toHaveTextContent('Docker Hub');
    expect(screen.getByRole('textbox', { name: 'Image Reference' })).toHaveValue('redis');
    expect(screen.getByText(/No Service has been created yet/)).toBeVisible();
  });

  it('loads an unmanaged Service adoption draft for review', async () => {
    const { user } = renderCitadel(
      <Routes>
        <Route path="/platforms/:platformId/services/add" element={<SwarmServiceForm mode="add" />} />
      </Routes>,
      { route: `/platforms/${platformId}/services/add?adoptFrom=service-1` },
    );

    expect(await screen.findByRole('textbox', { name: 'Name' })).toHaveValue('external-web');
    expect(screen.getByRole('textbox', { name: 'Description' })).toHaveValue(
      'Adopted from Docker Swarm Service external-web.',
    );
    const imageReference = screen.getByRole('textbox', { name: 'Image Reference' });
    const autoUpdate = screen.getByRole('combobox', { name: 'Auto Update' });
    expect(imageReference).toHaveValue('redis@sha256:344e3945a0b431c8ff1eecd58c5573538126bd756f02fc7e218ddf1fc2546366');
    expect(autoUpdate).toBeDisabled();
    expect(screen.getByText('Auto update is unavailable for an image pinned by digest.')).toBeVisible();
    expect(screen.getByRole('combobox', { name: 'Platform' })).toBeDisabled();
    expect(screen.getByRole('combobox', { name: 'Image Source' })).toBeDisabled();
    expect(screen.getByRole('textbox', { name: '# KEY=value' })).toHaveValue('{"team":"platform"}');
    const adoptButtons = screen.getAllByRole('button', { name: 'Adopt Service' });
    expect(adoptButtons).not.toHaveLength(0);
    expect(adoptButtons.every((button) => button.hasAttribute('disabled'))).toBe(true);

    await user.clear(imageReference);
    await user.type(imageReference, 'redis');

    expect(autoUpdate).toBeEnabled();
    expect(screen.queryByText('Auto update is unavailable for an image pinned by digest.')).not.toBeInTheDocument();
    await user.click(autoUpdate);
    await user.click(screen.getByText('Notify only'));
    expect(autoUpdate).toHaveTextContent('Notify only');

    await user.click(screen.getByRole('combobox', { name: 'Registry' }));
    await user.click(screen.getByText('Docker Hub'));

    expect(
      screen.getAllByRole('button', { name: 'Adopt Service' }).every((button) => !button.hasAttribute('disabled')),
    ).toBe(true);
  }, 10_000);
});
