import { useEffect, useMemo, useRef, useState } from 'react';
import { useParams, useSearchParams } from 'react-router';
import {
  CreateSwarmServiceInput,
  LicenseCapability,
  LookupResourceType,
  ManagedSwarmServiceView,
  PlatformType,
  SwarmServiceConfigReference,
  SwarmServiceImageInfoSwarmBuildImage,
  SwarmServiceImageInfoSwarmExternalImage,
  SwarmServiceRestartCondition,
  SwarmServiceResources,
  SwarmServiceSchedulingMode,
  SwarmServiceSecretReference,
  SwarmServiceSpec,
  SwarmNetworkView,
  SwarmServiceUpdateFailureAction,
  SwarmServiceUpdateOrder,
  UpdateBehavior,
} from '@/api/generated/api.types';
import {
  defineField,
  defineGroupField,
  defineRowField,
  defineSection,
  FieldInput,
  FieldSwitch,
  FieldTextArea,
  FormShell,
  ItemSelector,
} from '@/components/custom/form-builder';
import { MultiResourceSelectorField, ResourceSelectorField } from '@/components/custom/common';
import { AlertMessage } from '@/components/custom/alert-message';
import { MonacoToArrayEditor } from '@/lib/monaco';
import { ResourceTagSelector } from '@/features/tags/components';
import { WebhookConfigField } from '@/components/custom/webhook-config-field';
import { useMutate, useRead, useSaveResource } from '@/lib/hooks';
import { createEnvironmentVariableValidator } from '@/lib/utils';
import { ServiceMountsField, ServicePortsField, ServiceReferencesField } from './service-fields';
import { useLicenseEntitlements } from '@/features/license/use-license-entitlements';
import {
  findResourceProfile,
  ResourceProfileSelector,
  resourceProfiles,
  type ResourceProfile,
} from '@/components/custom/resource-profile-selector';

type FormValue = CreateSwarmServiceInput & { rowVersion?: number | string };

const EMPTY_RESOURCE_BINDING_LOOKUP: { name: string }[] = [];
const NANO_CPUS_PER_CPU = 1_000_000_000;
const BYTES_PER_MEBIBYTE = 1024 * 1024;

export const getSwarmResourceProfile = (resources: SwarmServiceResources | null | undefined): ResourceProfile => {
  if (resources?.reservationNanoCpus != null || resources?.reservationMemoryBytes != null) return 'automatic';

  return findResourceProfile(
    resources?.limitNanoCpus == null ? null : Number(resources.limitNanoCpus) / NANO_CPUS_PER_CPU,
    resources?.limitMemoryBytes == null ? null : Number(resources.limitMemoryBytes) / BYTES_PER_MEBIBYTE,
  );
};

export const getSwarmResourcesForProfile = (profile: ResourceProfile): SwarmServiceResources | null => {
  if (profile === 'automatic') return null;

  const limits = resourceProfiles[profile];
  if (limits.cpuCores == null || limits.memoryMiB == null) return null;

  return {
    limitNanoCpus: limits.cpuCores * NANO_CPUS_PER_CPU,
    limitMemoryBytes: limits.memoryMiB * BYTES_PER_MEBIBYTE,
    reservationNanoCpus: null,
    reservationMemoryBytes: null,
  };
};

const imageSources = {
  External: { label: 'External', description: 'Pull a tagged image from a Registry.' },
  Build: { label: 'Build', description: 'Use the latest successful output from a Build Project.' },
};

const schedulingModes = {
  Replicated: { label: 'Replicated', description: 'Run a fixed number of replicas.' },
  Global: { label: 'Global', description: 'Run one task on every eligible node.' },
};

const updateBehaviors = {
  Disabled: { label: 'Disabled', description: 'Do not check this image for updates.' },
  Notify: { label: 'Notify only', description: 'Report when a newer image digest is available.' },
  AutoDeploy: { label: 'Auto deploy', description: 'Apply an available image update automatically.' },
};

const restartConditions = {
  None: { label: 'Never', description: 'Do not restart failed tasks.' },
  OnFailure: { label: 'On failure', description: 'Restart only tasks that fail.' },
  Any: { label: 'Always', description: 'Restart tasks after any exit.' },
};

const updateOrders = {
  StopFirst: { label: 'Stop first', description: 'Stop the old task before starting its replacement.' },
  StartFirst: { label: 'Start first', description: 'Start the replacement before stopping the old task.' },
};

const failureActions = {
  Pause: { label: 'Pause', description: 'Pause the rollout after a task failure.' },
  Continue: { label: 'Continue', description: 'Continue the rollout after a task failure.' },
  Rollback: { label: 'Rollback', description: 'Ask Swarm to roll back the failed update.' },
};

const defaultSpec = (): SwarmServiceSpec => ({
  image: { $type: 'External', registryId: '', imageTag: '' },
  updateBehavior: UpdateBehavior.Disabled,
  schedulingMode: SwarmServiceSchedulingMode.Replicated,
  replicas: 1,
  command: [],
  arguments: [],
  environment: [],
  ports: [],
  networkIds: [],
  mounts: [],
  secrets: [],
  configs: [],
  placementConstraints: [],
});

export const getServiceNetworkOptions = (
  networks: SwarmNetworkView[],
  selectedIds: string[] = [],
): SwarmNetworkView[] =>
  networks.filter(
    (network) => network.scope.toLowerCase() === 'swarm' && (!network.isIngress || selectedIds.includes(network.id)),
  );

export const SwarmServiceForm = ({
  mode,
  resource,
  disabled,
}: {
  mode: 'add' | 'edit';
  resource?: ManagedSwarmServiceView;
  disabled?: boolean;
}) => {
  const { id, platformId } = useParams();
  const [searchParams] = useSearchParams();
  const duplicateFrom = mode === 'add' ? searchParams.get('duplicateFrom') : null;
  const duplicateDraftLoadedRef = useRef<string | null>(null);
  const create = useMutate('createSwarmService');
  const updateService = useMutate('updateSwarmService');
  const [update, setUpdate] = useState<Partial<FormValue>>({});
  const platformsQuery = useRead('listPlatforms');
  const registriesQuery = useRead('listRegistries');
  const buildsQuery = useRead('listBuildProjects');
  const { data: resourceBindingLookupData } = useRead('lookup', {
    query: {
      TargetResourceType: LookupResourceType.ResourceBinding,
      SourceResourceType: mode === 'edit' ? LookupResourceType.SwarmService : undefined,
      SourceResourceId: mode === 'edit' ? id : undefined,
    },
  });
  const { data: duplicateDraftData, isFetching: isDuplicateDraftLoading } = useRead(
    'getSwarmServiceDuplicateDraft',
    { id: duplicateFrom ?? '' },
    { enabled: mode === 'add' && !!duplicateFrom },
  );
  const { hasCapability: hasLicenseCapability } = useLicenseEntitlements();
  const operationalGuardrailsEnabled = hasLicenseCapability(LicenseCapability.OperationalGuardrails);
  const automatedOperationsEnabled = hasLicenseCapability(LicenseCapability.AutomatedOperations);
  const licensedUpdateBehaviors = useMemo(
    () => ({
      ...updateBehaviors,
      [UpdateBehavior.AutoDeploy]: {
        ...updateBehaviors[UpdateBehavior.AutoDeploy],
        disabled: !operationalGuardrailsEnabled,
        requiredLicense: !operationalGuardrailsEnabled ? ('Team' as const) : undefined,
      },
    }),
    [operationalGuardrailsEnabled],
  );

  const swarmPlatforms = useMemo(
    () => (platformsQuery.data?.data.platforms ?? []).filter((item) => item.type === PlatformType.DockerSwarm),
    [platformsQuery.data?.data.platforms],
  );
  const registries = useMemo(
    () => registriesQuery.data?.data.registries ?? [],
    [registriesQuery.data?.data.registries],
  );
  const builds = useMemo(() => buildsQuery.data?.data.projects ?? [], [buildsQuery.data?.data.projects]);
  const requestedPlatformId = platformId ?? searchParams.get('platformId') ?? '';
  const duplicateDraft = duplicateDraftData?.data;
  const duplicateWarnings = duplicateDraft?.warnings ?? [];
  const original = useMemo<FormValue>(
    () => ({
      name: resource?.name ?? '',
      description: resource?.description ?? '',
      platformId:
        resource?.platformId ??
        (swarmPlatforms.some((item) => item.id === requestedPlatformId)
          ? requestedPlatformId
          : (swarmPlatforms[0]?.id ?? '')),
      spec: resource?.spec ?? defaultSpec(),
      tagIds: resource?.tags.map((tag) => tag.id) ?? [],
      rowVersion: resource?.rowVersion,
    }),
    [requestedPlatformId, resource, swarmPlatforms],
  );

  useEffect(() => {
    if (!duplicateFrom || !duplicateDraft?.draft || duplicateDraftLoadedRef.current === duplicateFrom) return;

    duplicateDraftLoadedRef.current = duplicateFrom;
    setUpdate(duplicateDraft.draft as Partial<FormValue>);
  }, [duplicateFrom, duplicateDraft?.draft]);

  const currentSpec = { ...original.spec, ...(update.spec ?? {}) } as SwarmServiceSpec;
  const currentPlatformId = String(update.platformId ?? original.platformId ?? '');
  const inventoryArgs = useMemo(() => ({ platformId: currentPlatformId }), [currentPlatformId]);
  const networksQuery = useRead('listSwarmNetworks', inventoryArgs, { enabled: !!currentPlatformId });
  const secretsQuery = useRead('listSwarmSecrets', inventoryArgs, { enabled: !!currentPlatformId });
  const configsQuery = useRead('listSwarmConfigs', inventoryArgs, { enabled: !!currentPlatformId });
  const networkItems = useMemo(() => networksQuery.data?.data.items ?? [], [networksQuery.data?.data.items]);
  const networks = useMemo(
    () => getServiceNetworkOptions(networkItems, currentSpec.networkIds),
    [currentSpec.networkIds, networkItems],
  );
  const selectedIngressNetworks = useMemo(
    () => networkItems.filter((network) => network.isIngress && currentSpec.networkIds.includes(network.id)),
    [currentSpec.networkIds, networkItems],
  );
  const secretOptions = useMemo(
    () => (secretsQuery.data?.data.items ?? []).map((secret) => ({ id: secret.id, name: secret.name })),
    [secretsQuery.data?.data.items],
  );
  const configOptions = useMemo(
    () => (configsQuery.data?.data.items ?? []).map((config) => ({ id: config.id, name: config.name })),
    [configsQuery.data?.data.items],
  );
  const configurationNames = useMemo(
    () => (resourceBindingLookupData?.data ?? EMPTY_RESOURCE_BINDING_LOOKUP).map((entry) => entry.name).sort(),
    [resourceBindingLookupData?.data],
  );
  const validateServiceEnvironmentVariable = useMemo(
    () => createEnvironmentVariableValidator(configurationNames, 'Service'),
    [configurationNames],
  );
  const imageType = currentSpec.image.$type ?? 'External';
  const updateBehaviorUnavailable =
    imageType !== 'External' ||
    (currentSpec.image as SwarmServiceImageInfoSwarmExternalImage).imageTag?.includes('@') === true;
  const updateBehaviorWarning =
    imageType === 'Build'
      ? 'Build images do not support Registry update checks.'
      : 'Auto update is unavailable for an image pinned by digest.';

  const { save, isPending } = useSaveResource<FormValue, any>({
    mode,
    basePath: currentPlatformId ? `platforms/${currentPlatformId}/services` : 'swarm-services',
    entityName: 'Swarm Service',
    onCreate: (payload) => create.mutateAsync({ data: toCreateInput(payload) }),
    onUpdate: (payload) =>
      updateService.mutateAsync({
        id: id!,
        data: { spec: prepareSwarmServiceSpecForWrite(payload.spec), rowVersion: resource!.rowVersion },
      }),
  });

  const schema = useMemo(
    () => ({
      general: defineSection<FormValue>({
        title: '',
        items: [
          ...(mode === 'add'
            ? [
                defineGroupField<FormValue>({
                  id: 'details',
                  label: 'Details',
                  items: [
                    defineField<FormValue, 'name'>({
                      key: 'name',
                      label: 'Name',
                      required: true,
                      description: 'Internal identifier for this managed Service.',
                      validate: (value) => (String(value ?? '').trim() ? null : 'Name is required.'),
                      render: (value, set) => (
                        <FieldInput
                          value={value}
                          placeholder="e.g. production-web"
                          onChange={(name) => set({ name })}
                        />
                      ),
                    }),
                    defineField<FormValue, 'description'>({
                      key: 'description',
                      label: 'Description',
                      description: 'Optional description of this Service.',
                      render: (value, set) => (
                        <FieldTextArea value={value ?? ''} onChange={(description) => set({ description })} />
                      ),
                    }),
                    defineField<FormValue, 'tagIds'>({
                      key: 'tagIds',
                      label: 'Tags',
                      description: 'Optional tags for filtering and grouping this Service.',
                      render: (value, set) => (
                        <ResourceTagSelector
                          value={value ?? []}
                          disabled={disabled}
                          onChange={(tagIds) => set({ tagIds })}
                        />
                      ),
                    }),
                  ],
                }),
              ]
            : []),
          defineField<FormValue, 'platformId'>({
            key: 'platformId',
            label: 'Platform',
            required: true,
            disabled: mode === 'edit',
            description:
              mode === 'edit'
                ? 'The Swarm platform cannot be changed after this Service is created.'
                : 'Select the Docker Swarm platform that will run this Service.',
            validate: (value) => (value ? null : 'A Swarm platform is required.'),
            render: (value, set) => (
              <ResourceSelectorField
                sourceType={LookupResourceType.SwarmService}
                targetType={LookupResourceType.Platform}
                selected={value}
                items={swarmPlatforms}
                onSelect={(selected) => set({ platformId: selected?.id ?? '' })}
                placeholder="Select Swarm platform"
                allowClear={false}
              />
            ),
          }),
          defineGroupField<FormValue>({
            id: 'scheduling',
            label: 'Scheduling',
            title: 'Scheduling',
            description: 'Choose how Swarm places and maintains Service tasks.',
            items: [
              defineField<FormValue, 'spec.schedulingMode'>({
                key: 'spec.schedulingMode',
                label: 'Mode',
                required: true,
                disabled: !!resource?.dockerServiceId,
                description: resource?.dockerServiceId
                  ? 'Scheduling mode cannot change after the first successful Apply.'
                  : 'Replicated runs a fixed task count; Global runs one task per eligible node.',
                render: (value, set) => (
                  <ItemSelector
                    value={value}
                    collection={schedulingModes}
                    onChange={(schedulingMode: SwarmServiceSchedulingMode) =>
                      set((previous) => ({
                        spec: {
                          ...previous.spec!,
                          schedulingMode,
                          replicas:
                            schedulingMode === SwarmServiceSchedulingMode.Global
                              ? null
                              : (previous.spec?.replicas ?? 1),
                        },
                      }))
                    }
                  />
                ),
              }),
              ...(currentSpec.schedulingMode === SwarmServiceSchedulingMode.Replicated
                ? [
                    defineField<FormValue, 'spec.replicas'>({
                      key: 'spec.replicas',
                      label: 'Replicas',
                      required: true,
                      description: 'Desired number of running tasks.',
                      validate: validateNonNegativeRequired('Replicas'),
                      render: (value, set) => (
                        <FieldInput
                          type="number"
                          min={0}
                          value={value ?? 1}
                          onChange={(replicas) =>
                            set((previous) => ({ spec: { ...previous.spec!, replicas: Number(replicas) } }))
                          }
                        />
                      ),
                    }),
                  ]
                : []),
            ],
          }),
          defineGroupField<FormValue>({
            id: 'image',
            label: 'Image',
            items: [
              defineField<FormValue, 'spec.image.$type'>({
                key: 'spec.image.$type',
                label: 'Image Source',
                required: true,
                description: 'Select where Citadel obtains the Service image.',
                render: (value, set) => (
                  <ItemSelector
                    value={value}
                    collection={imageSources}
                    onChange={(source: 'External' | 'Build') =>
                      set((previous) => ({
                        spec: {
                          ...previous.spec!,
                          updateBehavior: source === 'Build' ? UpdateBehavior.Disabled : previous.spec?.updateBehavior,
                          webhook: source === 'Build' ? null : previous.spec?.webhook,
                          image:
                            source === 'Build'
                              ? { $type: 'Build', buildProjectId: '' }
                              : { $type: 'External', registryId: '', imageTag: '' },
                        },
                      }))
                    }
                  />
                ),
              }),
              ...(imageType === 'External'
                ? [
                    defineRowField<FormValue>({
                      id: 'external-image',
                      gap: 'gap-8',
                      fields: [
                        defineField<FormValue, 'spec.image.registryId'>({
                          key: 'spec.image.registryId',
                          label: 'Registry',
                          required: true,
                          description: 'Select the Registry used to pull this image.',
                          validate: (value) => (value ? null : 'Registry is required.'),
                          render: (value, set) => (
                            <ResourceSelectorField
                              sourceType={LookupResourceType.SwarmService}
                              targetType={LookupResourceType.Registry}
                              selected={value}
                              items={registries}
                              onSelect={(selected) =>
                                set((previous) => ({
                                  spec: {
                                    ...previous.spec!,
                                    image: {
                                      $type: 'External',
                                      ...((previous.spec?.image as SwarmServiceImageInfoSwarmExternalImage) ?? {}),
                                      registryId: selected?.id ?? '',
                                    } satisfies SwarmServiceImageInfoSwarmExternalImage,
                                  },
                                }))
                              }
                              placeholder="Select Registry"
                              allowClear={false}
                            />
                          ),
                        }),
                        defineField<FormValue, 'spec.image.imageTag'>({
                          key: 'spec.image.imageTag',
                          label: 'Image Reference',
                          required: true,
                          description: 'Enter a tagged or digest-pinned image reference.',
                          validate: (value) => (String(value ?? '').trim() ? null : 'Image reference is required.'),
                          render: (value, set) => (
                            <FieldInput
                              value={value}
                              placeholder="e.g. nginx:latest"
                              onChange={(imageTag) =>
                                set((previous) => ({
                                  spec: {
                                    ...previous.spec!,
                                    image: {
                                      $type: 'External',
                                      ...((previous.spec?.image as SwarmServiceImageInfoSwarmExternalImage) ?? {}),
                                      imageTag,
                                    } satisfies SwarmServiceImageInfoSwarmExternalImage,
                                  },
                                }))
                              }
                            />
                          ),
                        }),
                      ],
                    }),
                  ]
                : [
                    defineField<FormValue, 'spec.image.buildProjectId'>({
                      key: 'spec.image.buildProjectId',
                      label: 'Build',
                      required: true,
                      description: 'Build Project whose latest successful image will be deployed.',
                      validate: (value) => (value ? null : 'Build is required.'),
                      render: (value, set) => (
                        <ResourceSelectorField
                          sourceType={LookupResourceType.SwarmService}
                          targetType={LookupResourceType.Build}
                          selected={value}
                          items={builds}
                          onSelect={(selected) =>
                            set((previous) => ({
                              spec: {
                                ...previous.spec!,
                                image: {
                                  $type: 'Build',
                                  ...((previous.spec?.image as SwarmServiceImageInfoSwarmBuildImage) ?? {}),
                                  buildProjectId: selected?.id ?? '',
                                } satisfies SwarmServiceImageInfoSwarmBuildImage,
                                updateBehavior: UpdateBehavior.Disabled,
                              },
                            }))
                          }
                          placeholder="Select Build"
                          allowClear={false}
                        />
                      ),
                    }),
                  ]),
            ],
          }),
          defineGroupField<FormValue>({
            id: 'networks',
            label: 'Networks',
            items: [
              defineField<FormValue, 'spec.networkIds'>({
                key: 'spec.networkIds',
                label: 'Overlay Networks',
                description: 'Attach tasks to one or more Swarm-scoped overlay networks.',
                render: (value, set) => (
                  <div className="flex flex-col gap-2">
                    <MultiResourceSelectorField
                      sourceType={LookupResourceType.SwarmService}
                      targetType={LookupResourceType.Network}
                      selected={value ?? []}
                      items={networks}
                      onSelect={(selected) =>
                        set((previous) => ({
                          spec: { ...previous.spec!, networkIds: selected.map((network) => network.id) },
                        }))
                      }
                      placeholder="Select Network(s)"
                    />
                    {selectedIngressNetworks.length > 0 && (
                      <AlertMessage type="warning" title="Remove the ingress network">
                        Docker attaches this reserved network automatically. Remove it and save the Service before
                        applying.
                      </AlertMessage>
                    )}
                  </div>
                ),
              }),
              defineField<FormValue, 'spec.ports'>({
                key: 'spec.ports',
                label: 'Published Ports',
                description: 'Ingress uses the routing mesh. Host publishes only on nodes running a task.',
                validate: validatePorts,
                render: (value, set) => (
                  <ServicePortsField
                    value={value ?? []}
                    disabled={disabled}
                    onChange={(ports) => set((previous) => ({ spec: { ...previous.spec!, ports } }))}
                  />
                ),
              }),
            ],
          }),
          defineField<FormValue, 'spec.mounts'>({
            key: 'spec.mounts',
            label: 'Mounts',
            description: 'Bind paths must exist on every eligible node. Local-driver volumes are node-local.',
            validate: validateMounts,
            render: (value, set) => (
              <ServiceMountsField
                value={value ?? []}
                disabled={disabled}
                onChange={(mounts) => set((previous) => ({ spec: { ...previous.spec!, mounts } }))}
              />
            ),
          }),
          defineField<FormValue, 'spec.environment'>({
            key: 'spec.environment',
            label: 'Environment',
            description:
              'Select the environment keys injected into each task. Use KEY for a matching Citadel variable or KEY=${OTHER_KEY} to map a value.',
            render: (value, set) => (
              <MonacoToArrayEditor
                value={value ?? []}
                helperText="# LOG_LEVEL=${LOG_LEVEL}"
                language="key_value"
                completionItems={configurationNames}
                completionItemDetail="Citadel variable or secret"
                validateItem={validateServiceEnvironmentVariable}
                onChange={(environment) =>
                  set((previous) => ({ spec: { ...previous.spec!, environment: environment ?? [] } }))
                }
              />
            ),
          }),
          defineGroupField<FormValue>({
            id: 'swarm-data',
            label: 'Secrets and Configs',
            title: 'Secrets and Configs',
            description: 'Docker Swarm Secrets and Configs are separate from Citadel encrypted bindings.',
            items: [
              defineField<FormValue, 'spec.secrets'>({
                key: 'spec.secrets',
                label: 'Secrets',
                render: (value, set) => (
                  <ServiceReferencesField
                    kind="secret"
                    resources={secretOptions}
                    value={value ?? []}
                    disabled={disabled || secretsQuery.isLoading}
                    onChange={(secrets) =>
                      set((previous) => ({
                        spec: { ...previous.spec!, secrets: secrets as SwarmServiceSecretReference[] },
                      }))
                    }
                  />
                ),
              }),
              defineField<FormValue, 'spec.configs'>({
                key: 'spec.configs',
                label: 'Configs',
                render: (value, set) => (
                  <ServiceReferencesField
                    kind="config"
                    resources={configOptions}
                    value={value ?? []}
                    disabled={disabled || configsQuery.isLoading}
                    onChange={(configs) =>
                      set((previous) => ({
                        spec: { ...previous.spec!, configs: configs as SwarmServiceConfigReference[] },
                      }))
                    }
                  />
                ),
              }),
            ],
          }),
          defineField<FormValue, 'spec.updateBehavior'>({
            key: 'spec.updateBehavior',
            label: 'Auto Update',
            description: 'Define how Citadel handles a new image digest.',
            render: (value, set) => (
              <div className="flex flex-col gap-2">
                <ItemSelector
                  value={value}
                  disabled={updateBehaviorUnavailable}
                  collection={licensedUpdateBehaviors}
                  onChange={(updateBehavior: UpdateBehavior) =>
                    set((previous) => ({
                      spec: {
                        ...previous.spec!,
                        updateBehavior,
                        webhook:
                          updateBehavior === UpdateBehavior.Disabled && previous.spec?.webhook?.enabled
                            ? { ...previous.spec.webhook, enabled: false }
                            : previous.spec?.webhook,
                      },
                    }))
                  }
                />
                {updateBehaviorUnavailable && (
                  <AlertMessage type="warning" title="">
                    {updateBehaviorWarning}
                  </AlertMessage>
                )}
              </div>
            ),
          }),
        ],
      }),
      Advanced: defineSection<FormValue>({
        title: 'Advanced',
        items: [
          defineField<FormValue, 'spec.resources'>({
            key: 'spec.resources',
            label: 'Resources',
            description: 'Choose how much CPU and memory to allocate to each task.',
            render: (value, set) => (
              <ResourceProfileSelector
                value={getSwarmResourceProfile(value)}
                disabled={disabled}
                onChange={(profile) =>
                  set((previous) => ({
                    spec: { ...previous.spec!, resources: getSwarmResourcesForProfile(profile) },
                  }))
                }
              />
            ),
          }),
          ...(imageType === 'External'
            ? [
                defineGroupField<FormValue>({
                  id: 'webhook',
                  label: 'Webhook',
                  title: 'Webhook',
                  description: 'Trigger an image update check from a Git provider, CI system, or external caller.',
                  disabled: !automatedOperationsEnabled,
                  requiredLicense: automatedOperationsEnabled ? undefined : 'Team',
                  items: [
                    defineField<FormValue, 'spec.webhook'>({
                      key: 'spec.webhook',
                      label: 'Enabled',
                      render: (value, set) => (
                        <WebhookConfigField
                          resourceType="swarm-service"
                          resourceId={id}
                          execution="update"
                          showBranchFilter={false}
                          value={value ?? { enabled: false }}
                          disabled={disabled}
                          enableDisabled={!automatedOperationsEnabled}
                          onChange={(webhook) =>
                            set((previous) => ({
                              spec: {
                                ...previous.spec!,
                                webhook,
                                updateBehavior:
                                  webhook.enabled && previous.spec?.updateBehavior === UpdateBehavior.Disabled
                                    ? UpdateBehavior.Notify
                                    : previous.spec?.updateBehavior,
                              },
                            }))
                          }
                        />
                      ),
                    }),
                  ],
                }),
              ]
            : []),
          defineField<FormValue, 'spec.placementConstraints'>({
            key: 'spec.placementConstraints',
            label: 'Placement Constraints',
            description: 'Configure one Docker placement constraint per line.',
            render: (value, set) => (
              <MonacoToArrayEditor
                value={value ?? []}
                helperText="# node.labels.region==eu-west"
                language="string_list"
                onChange={(placementConstraints) =>
                  set((previous) => ({
                    spec: { ...previous.spec!, placementConstraints: placementConstraints ?? [] },
                  }))
                }
              />
            ),
          }),
          defineGroupField<FormValue>({
            id: 'process',
            label: 'Process',
            title: 'Process',
            description: 'Override the image process and task execution context.',
            items: [
              defineField<FormValue, 'spec.command'>({
                key: 'spec.command',
                label: 'Command',
                description: 'Override the image entrypoint, one argument per line.',
                render: (value, set) => (
                  <MonacoToArrayEditor
                    value={value ?? []}
                    helperText="# /usr/bin/server"
                    language="string_list"
                    onChange={(command) => set((previous) => ({ spec: { ...previous.spec!, command: command ?? [] } }))}
                  />
                ),
              }),
              defineField<FormValue, 'spec.arguments'>({
                key: 'spec.arguments',
                label: 'Arguments',
                description: 'Arguments passed to the Service command, one per line.',
                render: (value, set) => (
                  <MonacoToArrayEditor
                    value={value ?? []}
                    helperText="# --port=8080"
                    language="string_list"
                    onChange={(argumentsValue) =>
                      set((previous) => ({
                        spec: { ...previous.spec!, arguments: argumentsValue ?? [] },
                      }))
                    }
                  />
                ),
              }),
              defineField<FormValue, 'spec.user'>({
                key: 'spec.user',
                label: 'User',
                description: 'Optional user or UID used to run the task process.',
                render: (value, set) => (
                  <FieldInput
                    value={value ?? ''}
                    placeholder="e.g. 1000:1000"
                    onChange={(user) => set((previous) => ({ spec: { ...previous.spec!, user: user || null } }))}
                  />
                ),
              }),
              defineField<FormValue, 'spec.workingDirectory'>({
                key: 'spec.workingDirectory',
                label: 'Working Directory',
                description: 'Optional working directory inside the task container.',
                render: (value, set) => (
                  <FieldInput
                    value={value ?? ''}
                    placeholder="e.g. /app"
                    onChange={(workingDirectory) =>
                      set((previous) => ({
                        spec: { ...previous.spec!, workingDirectory: workingDirectory || null },
                      }))
                    }
                  />
                ),
              }),
            ],
          }),
          defineGroupField<FormValue>({
            id: 'health-check',
            label: 'Health Check',
            title: 'Health Check',
            description: 'Override the image health check for every Service task.',
            items: [
              defineField<FormValue, 'spec.healthCheck'>({
                key: 'spec.healthCheck',
                label: 'Enabled',
                render: (value, set) => (
                  <FieldSwitch
                    id="swarm-service-health-check"
                    checked={!!value}
                    onChange={(enabled) =>
                      set((previous) => ({
                        spec: {
                          ...previous.spec!,
                          healthCheck: enabled ? { test: ['CMD', 'true'] } : null,
                        },
                      }))
                    }
                  />
                ),
              }),
              ...(currentSpec.healthCheck
                ? [
                    defineField<FormValue, 'spec.healthCheck.test'>({
                      key: 'spec.healthCheck.test',
                      label: 'Test',
                      description: 'Health command in Docker test-array form.',
                      render: (value, set) => (
                        <MonacoToArrayEditor
                          value={value ?? []}
                          helperText="# CMD\ncurl\n-f\nhttp://localhost/health"
                          language="string_list"
                          onChange={(test) =>
                            set((previous) => ({
                              spec: {
                                ...previous.spec!,
                                healthCheck: { ...previous.spec!.healthCheck!, test: test ?? [] },
                              },
                            }))
                          }
                        />
                      ),
                    }),
                    defineField<FormValue, 'spec.healthCheck.intervalNanoseconds'>({
                      key: 'spec.healthCheck.intervalNanoseconds',
                      label: 'Interval',
                      description: 'Seconds between checks.',
                      validate: validateNonNegativeOptional('Health check interval'),
                      render: (value, set) => (
                        <FieldInput
                          type="number"
                          min={0}
                          value={nanosecondsToSeconds(value)}
                          placeholder="Seconds"
                          onChange={(item) => setHealthNumber(set, 'intervalNanoseconds', item)}
                        />
                      ),
                    }),
                    defineField<FormValue, 'spec.healthCheck.timeoutNanoseconds'>({
                      key: 'spec.healthCheck.timeoutNanoseconds',
                      label: 'Timeout',
                      description: 'Seconds before a check is considered failed.',
                      validate: validateNonNegativeOptional('Health check timeout'),
                      render: (value, set) => (
                        <FieldInput
                          type="number"
                          min={0}
                          value={nanosecondsToSeconds(value)}
                          placeholder="Seconds"
                          onChange={(item) => setHealthNumber(set, 'timeoutNanoseconds', item)}
                        />
                      ),
                    }),
                    defineField<FormValue, 'spec.healthCheck.retries'>({
                      key: 'spec.healthCheck.retries',
                      label: 'Retries',
                      description: 'Consecutive failures required before the task is unhealthy.',
                      validate: validateNonNegativeOptional('Health check retries'),
                      render: (value, set) => (
                        <FieldInput
                          type="number"
                          min={0}
                          value={value ?? ''}
                          onChange={(retries) =>
                            set((previous) => ({
                              spec: {
                                ...previous.spec!,
                                healthCheck: {
                                  ...previous.spec!.healthCheck!,
                                  retries: optionalNumber(retries),
                                },
                              },
                            }))
                          }
                        />
                      ),
                    }),
                    defineField<FormValue, 'spec.healthCheck.startPeriodNanoseconds'>({
                      key: 'spec.healthCheck.startPeriodNanoseconds',
                      label: 'Start Period',
                      description: 'Startup grace period in seconds.',
                      validate: validateNonNegativeOptional('Health check start period'),
                      render: (value, set) => (
                        <FieldInput
                          type="number"
                          min={0}
                          value={nanosecondsToSeconds(value)}
                          placeholder="Seconds"
                          onChange={(item) => setHealthNumber(set, 'startPeriodNanoseconds', item)}
                        />
                      ),
                    }),
                  ]
                : []),
            ],
          }),
          defineGroupField<FormValue>({
            id: 'lifecycle',
            label: 'Lifecycle',
            title: 'Lifecycle',
            description: 'Configure graceful shutdown and task restart behavior.',
            items: [
              defineField<FormValue, 'spec.stopGracePeriodNanoseconds'>({
                key: 'spec.stopGracePeriodNanoseconds',
                label: 'Stop Grace Period',
                description: 'Seconds allowed for a task to stop cleanly.',
                validate: validateNonNegativeOptional('Stop grace period'),
                render: (value, set) => (
                  <FieldInput
                    type="number"
                    min={0}
                    value={nanosecondsToSeconds(value)}
                    placeholder="Seconds"
                    onChange={(item) =>
                      set((previous) => ({
                        spec: {
                          ...previous.spec!,
                          stopGracePeriodNanoseconds: secondsToNanoseconds(item),
                        },
                      }))
                    }
                  />
                ),
              }),
              defineField<FormValue, 'spec.restartPolicy'>({
                key: 'spec.restartPolicy',
                label: 'Restart Policy',
                render: (value, set) => (
                  <FieldSwitch
                    id="swarm-service-restart-policy"
                    checked={!!value}
                    onChange={(enabled) =>
                      set((previous) => ({
                        spec: {
                          ...previous.spec!,
                          restartPolicy: enabled ? { condition: SwarmServiceRestartCondition.Any } : null,
                        },
                      }))
                    }
                  />
                ),
              }),
              ...(currentSpec.restartPolicy
                ? [
                    defineField<FormValue, 'spec.restartPolicy.condition'>({
                      key: 'spec.restartPolicy.condition',
                      label: 'Restart Condition',
                      render: (value, set) => (
                        <ItemSelector
                          value={value}
                          collection={restartConditions}
                          onChange={(condition: SwarmServiceRestartCondition) =>
                            set((previous) => ({
                              spec: {
                                ...previous.spec!,
                                restartPolicy: { ...previous.spec!.restartPolicy!, condition },
                              },
                            }))
                          }
                        />
                      ),
                    }),
                    defineField<FormValue, 'spec.restartPolicy.delayNanoseconds'>({
                      key: 'spec.restartPolicy.delayNanoseconds',
                      label: 'Restart Delay',
                      description: 'Seconds before restarting a task.',
                      validate: validateNonNegativeOptional('Restart delay'),
                      render: (value, set) => (
                        <FieldInput
                          type="number"
                          min={0}
                          value={nanosecondsToSeconds(value)}
                          placeholder="Seconds"
                          onChange={(item) => setRestartNumber(set, 'delayNanoseconds', item)}
                        />
                      ),
                    }),
                    defineField<FormValue, 'spec.restartPolicy.maximumAttempts'>({
                      key: 'spec.restartPolicy.maximumAttempts',
                      label: 'Maximum Attempts',
                      validate: validateNonNegativeOptional('Maximum attempts'),
                      render: (value, set) => (
                        <FieldInput
                          type="number"
                          min={0}
                          value={value ?? ''}
                          onChange={(maximumAttempts) =>
                            set((previous) => ({
                              spec: {
                                ...previous.spec!,
                                restartPolicy: {
                                  ...previous.spec!.restartPolicy!,
                                  maximumAttempts: optionalNumber(maximumAttempts),
                                },
                              },
                            }))
                          }
                        />
                      ),
                    }),
                    defineField<FormValue, 'spec.restartPolicy.windowNanoseconds'>({
                      key: 'spec.restartPolicy.windowNanoseconds',
                      label: 'Evaluation Window',
                      description: 'Restart evaluation window in seconds.',
                      validate: validateNonNegativeOptional('Restart window'),
                      render: (value, set) => (
                        <FieldInput
                          type="number"
                          min={0}
                          value={nanosecondsToSeconds(value)}
                          placeholder="Seconds"
                          onChange={(item) => setRestartNumber(set, 'windowNanoseconds', item)}
                        />
                      ),
                    }),
                  ]
                : []),
            ],
          }),
          defineGroupField<FormValue>({
            id: 'update-policy',
            label: 'Rolling Update',
            title: 'Rolling Update',
            description: 'Control how Swarm replaces tasks when the Service changes.',
            items: [
              defineField<FormValue, 'spec.updatePolicy'>({
                key: 'spec.updatePolicy',
                label: 'Enabled',
                render: (value, set) => (
                  <FieldSwitch
                    id="swarm-service-update-policy"
                    checked={!!value}
                    onChange={(enabled) =>
                      set((previous) => ({
                        spec: {
                          ...previous.spec!,
                          updatePolicy: enabled
                            ? {
                                parallelism: 1,
                                order: SwarmServiceUpdateOrder.StopFirst,
                                failureAction: SwarmServiceUpdateFailureAction.Pause,
                              }
                            : null,
                        },
                      }))
                    }
                  />
                ),
              }),
              ...(currentSpec.updatePolicy
                ? [
                    defineField<FormValue, 'spec.updatePolicy.parallelism'>({
                      key: 'spec.updatePolicy.parallelism',
                      label: 'Parallelism',
                      description: 'Maximum number of tasks updated simultaneously.',
                      validate: validateNonNegativeRequired('Parallelism'),
                      render: (value, set) => (
                        <FieldInput
                          type="number"
                          min={0}
                          value={value ?? 1}
                          onChange={(parallelism) =>
                            set((previous) => ({
                              spec: {
                                ...previous.spec!,
                                updatePolicy: {
                                  ...previous.spec!.updatePolicy!,
                                  parallelism: Number(parallelism),
                                },
                              },
                            }))
                          }
                        />
                      ),
                    }),
                    defineField<FormValue, 'spec.updatePolicy.delayNanoseconds'>({
                      key: 'spec.updatePolicy.delayNanoseconds',
                      label: 'Delay',
                      description: 'Seconds between update batches.',
                      validate: validateNonNegativeOptional('Update delay'),
                      render: (value, set) => (
                        <FieldInput
                          type="number"
                          min={0}
                          value={nanosecondsToSeconds(value)}
                          placeholder="Seconds"
                          onChange={(delay) =>
                            set((previous) => ({
                              spec: {
                                ...previous.spec!,
                                updatePolicy: {
                                  ...previous.spec!.updatePolicy!,
                                  delayNanoseconds: secondsToNanoseconds(delay),
                                },
                              },
                            }))
                          }
                        />
                      ),
                    }),
                    defineField<FormValue, 'spec.updatePolicy.order'>({
                      key: 'spec.updatePolicy.order',
                      label: 'Order',
                      render: (value, set) => (
                        <ItemSelector
                          value={value}
                          collection={updateOrders}
                          onChange={(order: SwarmServiceUpdateOrder) =>
                            set((previous) => ({
                              spec: {
                                ...previous.spec!,
                                updatePolicy: { ...previous.spec!.updatePolicy!, order },
                              },
                            }))
                          }
                        />
                      ),
                    }),
                    defineField<FormValue, 'spec.updatePolicy.failureAction'>({
                      key: 'spec.updatePolicy.failureAction',
                      label: 'Failure Action',
                      render: (value, set) => (
                        <ItemSelector
                          value={value}
                          collection={failureActions}
                          onChange={(failureAction: SwarmServiceUpdateFailureAction) =>
                            set((previous) => ({
                              spec: {
                                ...previous.spec!,
                                updatePolicy: { ...previous.spec!.updatePolicy!, failureAction },
                              },
                            }))
                          }
                        />
                      ),
                    }),
                  ]
                : []),
            ],
          }),
        ],
      }),
    }),
    [
      builds,
      automatedOperationsEnabled,
      configurationNames,
      configOptions,
      configsQuery.isLoading,
      currentSpec.healthCheck,
      currentSpec.restartPolicy,
      currentSpec.schedulingMode,
      currentSpec.updatePolicy,
      disabled,
      imageType,
      id,
      licensedUpdateBehaviors,
      mode,
      networks,
      registries,
      resource?.dockerServiceId,
      selectedIngressNetworks.length,
      secretOptions,
      secretsQuery.isLoading,
      swarmPlatforms,
      updateBehaviorUnavailable,
      updateBehaviorWarning,
      validateServiceEnvironmentVariable,
    ],
  );

  const formDraftKey = duplicateFrom ? `swarm-service:duplicate:${duplicateFrom}` : `SwarmService:${id ?? 'new'}`;

  return (
    <div className="flex flex-col gap-3">
      {duplicateFrom && (
        <AlertMessage type="info" title={isDuplicateDraftLoading ? 'Loading duplicate draft' : 'Duplicate draft'}>
          No Service has been created yet. Review the copied configuration, then save it to create the Service.
        </AlertMessage>
      )}
      {duplicateWarnings.map((warning) => (
        <AlertMessage key={`${warning.code}:${warning.fieldPath ?? ''}`} type="warning" title={warning.code}>
          {warning.message}
        </AlertMessage>
      ))}
      <FormShell
        schema={schema}
        original={original}
        update={update}
        setUpdate={setUpdate}
        onSave={save}
        pending={isPending}
        disabled={disabled}
        mode={mode}
        draftKey={formDraftKey}
        draftVersion={resource?.rowVersion}
      />
    </div>
  );
};

const optionalNumber = (value: string | number | undefined | null): number | null =>
  value === '' || value == null ? null : Number(value);

const secondsToNanoseconds = (value: string | number | undefined | null): number | null => {
  const seconds = optionalNumber(value);
  return seconds == null ? null : seconds * 1_000_000_000;
};

const nanosecondsToSeconds = (value: string | number | undefined | null): number | '' => {
  const nanoseconds = optionalNumber(value);
  return nanoseconds == null ? '' : nanoseconds / 1_000_000_000;
};

const validateNonNegativeOptional = (label: string) => (value: unknown) => {
  if (value === '' || value == null) return null;
  return Number.isFinite(Number(value)) && Number(value) >= 0 ? null : `${label} cannot be negative.`;
};

const validateNonNegativeRequired = (label: string) => (value: unknown) => {
  if (value === '' || value == null) return `${label} is required.`;
  return Number.isFinite(Number(value)) && Number(value) >= 0 ? null : `${label} cannot be negative.`;
};

const validatePorts = (
  value: Array<{ targetPort: number | string; publishedPort?: number | string | null }> | undefined,
) =>
  value?.some(
    (port) =>
      Number(port.targetPort) < 1 ||
      Number(port.targetPort) > 65535 ||
      (port.publishedPort != null && (Number(port.publishedPort) < 1 || Number(port.publishedPort) > 65535)),
  )
    ? 'Ports must be between 1 and 65535.'
    : null;

const validateMounts = (value: Array<{ source: string; target: string }> | undefined) =>
  value?.some((mount) => !mount.source.trim() || !mount.target.trim())
    ? 'Every mount requires a source and target.'
    : null;

const setHealthNumber = (
  set: (change: (previous: Partial<FormValue>) => Partial<FormValue>) => void,
  key: 'intervalNanoseconds' | 'timeoutNanoseconds' | 'startPeriodNanoseconds',
  value: string | number,
) =>
  set((previous) => ({
    spec: {
      ...previous.spec!,
      healthCheck: { ...previous.spec!.healthCheck!, [key]: secondsToNanoseconds(value) },
    },
  }));

const setRestartNumber = (
  set: (change: (previous: Partial<FormValue>) => Partial<FormValue>) => void,
  key: 'delayNanoseconds' | 'windowNanoseconds',
  value: string | number,
) =>
  set((previous) => ({
    spec: {
      ...previous.spec!,
      restartPolicy: { ...previous.spec!.restartPolicy!, [key]: secondsToNanoseconds(value) },
    },
  }));

const toCreateInput = (value: FormValue): CreateSwarmServiceInput => ({
  name: value.name.trim(),
  description: value.description?.trim() || null,
  platformId: value.platformId,
  spec: prepareSwarmServiceSpecForWrite(value.spec),
  tagIds: value.tagIds,
  duplicateSource: value.duplicateSource,
});

export const prepareSwarmServiceSpecForWrite = (spec: SwarmServiceSpec): SwarmServiceSpec => {
  const { $type, ...image } = spec.image;
  const imageType = $type ?? ('buildProjectId' in image ? 'Build' : 'External');
  return {
    ...spec,
    image: { $type: imageType, ...image } as SwarmServiceSpec['image'],
  };
};
