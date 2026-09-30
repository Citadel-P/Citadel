import {
  BackupExecutionLocation,
  BackupPolicyInput,
  BackupPolicyView,
  BackupRepositorySpecFileSystemBackupRepositorySpec,
  BackupRepositoryType,
  BackupRepositoryView,
  BackupSourceSpec,
  BackupSourceSpecDeploymentBackupSource,
  BackupSourceSpecDockerVolumeBackupSource,
  BackupSourceSpecStackBackupSource,
  BackupSourceSpecSwarmServiceBackupSource,
  BackupWebhookConfig,
  BackupSourceType,
  LicenseCapability,
  DeploymentBackupSourcePreviewView,
  VolumeView,
  LookupResourceType,
  PlatformConnectorType,
  PlatformType,
  PlatformView,
  StackBackupSourcePreviewView,
  SwarmServiceBackupSourcePreviewView,
  StackVolumeKind,
  UpdateBackupPolicyInput,
  VolumeBackupConsistency,
} from '@/api/generated/api.types';
import { ResourceSelectorField } from '@/components/custom/common';
import {
  defineField,
  defineGroupField,
  defineSection,
  FieldInput,
  FieldSwitch,
  FieldTextArea,
  FormShell,
  ItemSelector,
} from '@/components/custom/form-builder';
import { TimezoneSelectField } from '@/components/custom/timezone-select';
import { WebhookConfigField } from '@/components/custom/webhook-config-field';
import { Badge } from '@/components/ui/badge';
import { useLicenseEntitlements } from '@/features/license/use-license-entitlements';
import { ResourceTagSelector } from '@/features/tags/components';
import { byteTransform } from '@/lib/bytes.helper';
import { useMutate, useRead, useSaveResource } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { Box, Database, Info, Layers, LoaderCircle, TriangleAlert } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useParams } from 'react-router';

type BackupPolicyFormValue = Omit<BackupPolicyInput, 'runAsActorId'> & {
  id?: string;
  runAsActorId?: string | null;
  scheduleEnabled: boolean;
  webhook: BackupWebhookConfig | null;
  tagIds?: string[] | null;
};

const EMPTY_BACKUP_REPOSITORIES: BackupRepositoryView[] = [];

const sourceTypes = {
  CitadelSystem: {
    label: 'Citadel backup',
    description: "Back up Citadel's PostgreSQL database and required recovery keys.",
  },
  DockerVolume: {
    label: 'Docker volume',
    description: 'Back up one Docker volume from a selected platform.',
  },
  Stack: {
    label: 'Stack',
    description: 'Back up all resolved named volumes used by a stack.',
  },
  Deployment: {
    label: 'Deployment',
    description: 'Back up all resolved named volumes used by a deployment.',
  },
  SwarmService: {
    label: 'Swarm service',
    description: 'Back up local named volumes mounted by the current tasks of a managed Swarm service.',
  },
} as const;

const consistencyTypes = {
  Live: {
    label: 'Live',
    description: 'Run Restic without stopping containers that use the volume.',
  },
  StopAttachedContainers: {
    label: 'Stop attached containers',
    description: 'Stop affected containers during the volume snapshot and restart them afterward.',
  },
} as const;

const emptyPolicy = (): BackupPolicyFormValue => ({
  name: '',
  description: '',
  source: createCitadelSystemSource(),
  backupRepositoryId: '',
  enabled: true,
  scheduleEnabled: false,
  cron: '',
  timeZone: 'UTC',
  webhook: { enabled: false },
  keepLastSuccessful: 14,
  timeoutSeconds: 1800,
  alertOnFailure: true,
  runAsActorId: null,
  tagIds: [],
});

export function BackupPolicyForm({
  mode,
  resource,
  disabled,
  metadataChanged,
}: {
  mode: 'add' | 'edit';
  resource?: BackupPolicyView;
  disabled?: boolean;
  metadataChanged?: boolean;
}) {
  const { id } = useParams();
  const queryClient = useQueryClient();
  const createPolicy = useMutate('createBackupPolicy');
  const updatePolicy = useMutate('updateBackupPolicy');
  const [update, setUpdate] = useState<Partial<BackupPolicyFormValue>>({});
  const { hasCapability: hasLicenseCapability } = useLicenseEntitlements();
  const automatedOperationsEnabled = hasLicenseCapability(LicenseCapability.AutomatedOperations);

  const original = useMemo(() => toFormValue(resource), [resource]);
  const currentSource = useMemo(() => mergeSource(original.source, update.source), [original.source, update.source]);
  const currentSourceType = currentSource.$type ?? BackupSourceType.CitadelSystem;
  const currentScheduleEnabled = update.scheduleEnabled ?? original.scheduleEnabled;
  const currentPlatformId =
    currentSource.$type === BackupSourceType.DockerVolume
      ? String((currentSource as BackupSourceSpecDockerVolumeBackupSource).platformId ?? '')
      : '';
  const currentStackId =
    currentSource.$type === BackupSourceType.Stack
      ? String((currentSource as BackupSourceSpecStackBackupSource).stackId ?? '')
      : '';
  const currentDeploymentId =
    currentSource.$type === BackupSourceType.Deployment
      ? String((currentSource as BackupSourceSpecDeploymentBackupSource).deploymentId ?? '')
      : '';
  const currentSwarmServiceId =
    currentSource.$type === BackupSourceType.SwarmService
      ? String((currentSource as BackupSourceSpecSwarmServiceBackupSource).swarmServiceId ?? '')
      : '';
  const volumeListArgs = useMemo(() => ({ platformId: currentPlatformId, query: {} }), [currentPlatformId]);
  const volumeList = useRead('listVolumes', volumeListArgs, {
    enabled: currentSourceType === BackupSourceType.DockerVolume && Boolean(currentPlatformId),
  });
  const stackPreviewArgs = useMemo(() => ({ stackId: currentStackId }), [currentStackId]);
  const stackPreview = useRead('getStackBackupSourcePreview', stackPreviewArgs, {
    enabled: currentSourceType === BackupSourceType.Stack && Boolean(currentStackId),
  });
  const deploymentPreviewArgs = useMemo(() => ({ deploymentId: currentDeploymentId }), [currentDeploymentId]);
  const deploymentPreview = useRead('getDeploymentBackupSourcePreview', deploymentPreviewArgs, {
    enabled: currentSourceType === BackupSourceType.Deployment && Boolean(currentDeploymentId),
  });
  const swarmServicePreviewArgs = useMemo(() => ({ id: currentSwarmServiceId }), [currentSwarmServiceId]);
  const swarmServicePreview = useRead('getSwarmServiceBackupSourcePreview', swarmServicePreviewArgs, {
    enabled: currentSourceType === BackupSourceType.SwarmService && Boolean(currentSwarmServiceId),
  });
  const backupRepositories = useRead('listBackupRepositories');
  const currentBackupRepositoryId = update.backupRepositoryId ?? original.backupRepositoryId;
  const repositories = backupRepositories.data?.data.repositories ?? EMPTY_BACKUP_REPOSITORIES;
  const selectedRepository = useMemo(
    () => repositories.find((repository) => repository.id === currentBackupRepositoryId),
    [repositories, currentBackupRepositoryId],
  );
  const currentSourcePlatformId =
    currentSourceType === BackupSourceType.DockerVolume
      ? currentPlatformId
      : currentSourceType === BackupSourceType.Stack
        ? stackPreview.data?.data.platformId
        : currentSourceType === BackupSourceType.Deployment
          ? deploymentPreview.data?.data.platformId
          : currentSourceType === BackupSourceType.SwarmService
            ? swarmServicePreview.data?.data.platformId
            : undefined;
  const sourcePlatformArgs = useMemo(() => ({ id: currentSourcePlatformId ?? '' }), [currentSourcePlatformId]);
  const sourcePlatform = useRead('getPlatfom', sourcePlatformArgs, { enabled: Boolean(currentSourcePlatformId) });
  const isSwarmSourcePlatform = sourcePlatform.data?.data.type === PlatformType.DockerSwarm;
  const repositoryCompatibilityMessage = getRepositoryCompatibilityMessage(
    currentSourceType,
    selectedRepository,
    sourcePlatform.data?.data,
    currentSourcePlatformId,
  );

  const refreshData = useCallback(() => {
    localStorage.removeItem(`backup-policy:${id ?? 'new'}`);
    queryClient.invalidateQueries({ queryKey: ['listBackupPolicies'] });
    queryClient.invalidateQueries({ queryKey: ['listBackupRuns'] });
    queryClient.invalidateQueries({ queryKey: ['getPlatformBackupSummaries'] });
    if (id) {
      queryClient.invalidateQueries({ queryKey: ['getBackupPolicy', { id }] });
    }
  }, [id, queryClient]);

  useEffect(() => {
    if (!metadataChanged) return;
    refreshData();
  }, [metadataChanged, refreshData]);

  const { save: handleSave, isPending } = useSaveResource<BackupPolicyFormValue, any>({
    mode,
    basePath: 'backup-policies',
    entityName: 'Backup policy',
    onCreate: (payload) => createPolicy.mutateAsync({ data: toCreateInput(payload) } as any),
    onUpdate: (payload) => updatePolicy.mutateAsync({ id: id!, data: toUpdateInput(payload, update) } as any),
    onRefresh: refreshData,
  });

  const schema = useMemo(
    () => ({
      Config: defineSection<BackupPolicyFormValue>({
        title: '',
        items: [
          ...(mode === 'add'
            ? [
                defineGroupField<BackupPolicyFormValue>({
                  id: 'details',
                  label: 'Details',
                  items: [
                    defineField<BackupPolicyFormValue, 'name'>({
                      key: 'name',
                      label: 'Name',
                      required: true,
                      description: 'Unique name for this backup policy.',
                      validate: (value) => (!String(value ?? '').trim() ? 'Name is required' : null),
                      render: (value, set) => (
                        <FieldInput
                          value={value ?? ''}
                          placeholder="daily-citadel-backup"
                          disabled={disabled}
                          onChange={(name) => set({ name })}
                        />
                      ),
                    }),
                    defineField<BackupPolicyFormValue, 'description'>({
                      key: 'description',
                      label: 'Description',
                      required: false,
                      description: 'Optional notes for operators.',
                      render: (value, set) => (
                        <FieldTextArea
                          value={value ?? ''}
                          disabled={disabled}
                          onChange={(description) => set({ description })}
                        />
                      ),
                    }),
                    defineField<BackupPolicyFormValue, 'tagIds'>({
                      key: 'tagIds',
                      label: 'Tags',
                      required: false,
                      description: 'Optional tags for filtering and grouping backup policies.',
                      render: (value, set) => (
                        <ResourceTagSelector value={value} disabled={disabled} onChange={(tagIds) => set({ tagIds })} />
                      ),
                    }),
                  ],
                }),
              ]
            : []),

          defineGroupField<BackupPolicyFormValue>({
            id: 'source',
            label: 'Source',
            description:
              mode === 'edit' && resource?.firstSuccessfulRunAt
                ? 'The source is locked after the first successful run.'
                : undefined,
            items: [
              defineField<BackupPolicyFormValue, 'source.$type'>({
                key: 'source.$type',
                label: 'Type',
                required: true,
                disabled: isSourceLocked(resource),
                render: (value, set) => (
                  <div className="flex max-w-150 flex-col gap-2">
                    <ItemSelector
                      value={value ?? BackupSourceType.CitadelSystem}
                      collection={sourceTypes}
                      disabled={disabled || isSourceLocked(resource)}
                      onChange={(nextType: BackupSourceType) =>
                        set({
                          source:
                            nextType === BackupSourceType.DockerVolume
                              ? createDockerVolumeSource()
                              : nextType === BackupSourceType.Stack
                                ? createStackSource()
                                : nextType === BackupSourceType.Deployment
                                  ? createDeploymentSource()
                                  : nextType === BackupSourceType.SwarmService
                                    ? createSwarmServiceSource()
                                    : createCitadelSystemSource(),
                        })
                      }
                    />
                    {currentSourceType === BackupSourceType.CitadelSystem && (
                      <div className="flex items-start gap-2 rounded-sm border bg-muted/50 p-3">
                        <Info className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
                        <div className="min-w-0">
                          <div className="text-sm font-medium">Offline recovery</div>
                          <p className="text-xs text-muted-foreground">
                            The backup includes a PostgreSQL logical dump and file-backed recovery keys. Keys supplied
                            through external configuration are recorded in the manifest and must be preserved
                            separately. Restore while Citadel Core is stopped.
                          </p>
                        </div>
                      </div>
                    )}
                  </div>
                ),
              }),
              ...(currentSourceType === BackupSourceType.DockerVolume
                ? [
                    defineField<BackupPolicyFormValue, 'source.platformId'>({
                      key: 'source.platformId',
                      label: 'Platform',
                      required: true,
                      disabled: isSourceLocked(resource),
                      description: 'Platform that owns the Docker volume.',
                      render: (value, set) => (
                        <ResourceSelectorField
                          targetType={LookupResourceType.Platform}
                          selected={value}
                          disabled={disabled || isSourceLocked(resource)}
                          onSelect={(platform: PlatformView | undefined) =>
                            set((prev) => ({
                              source: {
                                ...(mergeSource(
                                  original.source,
                                  prev.source,
                                ) as BackupSourceSpecDockerVolumeBackupSource),
                                $type: BackupSourceType.DockerVolume,
                                platformId: platform?.id ?? '',
                                volumeName: '',
                                dockerNodeId: null,
                              },
                            }))
                          }
                          placeholder="Select Platform"
                        />
                      ),
                    }),
                    defineField<BackupPolicyFormValue, 'source.volumeName'>({
                      key: 'source.volumeName',
                      label: 'Volume',
                      required: true,
                      disabled: isSourceLocked(resource),
                      description: 'Docker volume name to back up.',
                      validate: (value) =>
                        currentSourceType === BackupSourceType.DockerVolume && !String(value ?? '').trim()
                          ? 'Volume name is required'
                          : null,
                      render: (value, set) => (
                        <VolumeSelector
                          value={value ?? ''}
                          dockerNodeId={
                            (currentSource as BackupSourceSpecDockerVolumeBackupSource).dockerNodeId ?? null
                          }
                          volumes={volumeList.data?.data.volumes ?? []}
                          isLoading={volumeList.isLoading || volumeList.isFetching}
                          hasPlatform={Boolean(currentPlatformId)}
                          disabled={disabled || isSourceLocked(resource) || !currentPlatformId}
                          isSwarm={isSwarmSourcePlatform}
                          onChange={(volume) =>
                            set({
                              source: {
                                volumeName: volume?.name ?? '',
                                dockerNodeId: volume?.dockerNodeId ?? null,
                              } as any,
                            })
                          }
                        />
                      ),
                    }),
                    ...(!isSwarmSourcePlatform
                      ? [
                          defineField<BackupPolicyFormValue, 'source.consistency'>({
                            key: 'source.consistency',
                            label: 'Consistency',
                            required: true,
                            disabled: isSourceLocked(resource),
                            description: 'How Citadel handles containers that use the selected volume.',
                            render: (value, set) => (
                              <ItemSelector
                                value={value ?? VolumeBackupConsistency.Live}
                                collection={consistencyTypes}
                                disabled={disabled || isSourceLocked(resource)}
                                onChange={(consistency: VolumeBackupConsistency) =>
                                  set({ source: { consistency } as any })
                                }
                              />
                            ),
                          }),
                        ]
                      : []),
                  ]
                : []),
              ...(currentSourceType === BackupSourceType.Stack
                ? [
                    defineField<BackupPolicyFormValue, 'source.stackId'>({
                      key: 'source.stackId',
                      label: 'Stack',
                      required: true,
                      disabled: isSourceLocked(resource),
                      description: 'Stack whose Docker named volumes will be backed up together.',
                      validate: (value) =>
                        currentSourceType === BackupSourceType.Stack && !String(value ?? '').trim()
                          ? 'Stack is required'
                          : null,
                      render: (value, set) => (
                        <div className="flex max-w-150 flex-col gap-4">
                          <ResourceSelectorField
                            targetType={LookupResourceType.Stack}
                            selected={value}
                            disabled={disabled || isSourceLocked(resource)}
                            onSelect={(stack: { id: string } | undefined) =>
                              set((prev) => ({
                                source: {
                                  ...(mergeSource(original.source, prev.source) as BackupSourceSpecStackBackupSource),
                                  $type: BackupSourceType.Stack,
                                  stackId: stack?.id ?? '',
                                },
                              }))
                            }
                            placeholder="Select Stack"
                          />
                          <StackSourcePreview
                            preview={stackPreview.data?.data}
                            isLoading={stackPreview.isLoading || stackPreview.isFetching}
                            hasSelection={Boolean(currentStackId)}
                          />
                        </div>
                      ),
                    }),
                  ]
                : []),
              ...(currentSourceType === BackupSourceType.Deployment
                ? [
                    defineField<BackupPolicyFormValue, 'source.deploymentId'>({
                      key: 'source.deploymentId',
                      label: 'Deployment',
                      required: true,
                      disabled: isSourceLocked(resource),
                      description: 'Deployment whose Docker named volumes will be backed up together.',
                      validate: (value) =>
                        currentSourceType === BackupSourceType.Deployment && !String(value ?? '').trim()
                          ? 'Deployment is required'
                          : null,
                      render: (value, set) => (
                        <div className="flex max-w-150 flex-col gap-4">
                          <ResourceSelectorField
                            targetType={LookupResourceType.Deployment}
                            selected={value}
                            disabled={disabled || isSourceLocked(resource)}
                            onSelect={(deployment: { id: string } | undefined) =>
                              set((prev) => ({
                                source: {
                                  ...(mergeSource(
                                    original.source,
                                    prev.source,
                                  ) as BackupSourceSpecDeploymentBackupSource),
                                  $type: BackupSourceType.Deployment,
                                  deploymentId: deployment?.id ?? '',
                                },
                              }))
                            }
                            placeholder="Select Deployment"
                          />
                          <DeploymentSourcePreview
                            preview={deploymentPreview.data?.data}
                            isLoading={deploymentPreview.isLoading || deploymentPreview.isFetching}
                            hasSelection={Boolean(currentDeploymentId)}
                          />
                        </div>
                      ),
                    }),
                  ]
                : []),
              ...(currentSourceType === BackupSourceType.SwarmService
                ? [
                    defineField<BackupPolicyFormValue, 'source.swarmServiceId'>({
                      key: 'source.swarmServiceId',
                      label: 'Swarm Service',
                      required: true,
                      disabled: isSourceLocked(resource),
                      description: 'Managed Swarm Service whose current task Volumes will be backed up.',
                      validate: (value) =>
                        currentSourceType === BackupSourceType.SwarmService && !String(value ?? '').trim()
                          ? 'Swarm Service is required'
                          : null,
                      render: (value, set) => (
                        <div className="flex max-w-150 flex-col gap-4">
                          <ResourceSelectorField
                            targetType={LookupResourceType.SwarmService}
                            selected={value}
                            disabled={disabled || isSourceLocked(resource)}
                            onSelect={(service: { id: string } | undefined) =>
                              set((prev) => ({
                                source: {
                                  ...(mergeSource(
                                    original.source,
                                    prev.source,
                                  ) as BackupSourceSpecSwarmServiceBackupSource),
                                  $type: BackupSourceType.SwarmService,
                                  swarmServiceId: service?.id ?? '',
                                },
                              }))
                            }
                            placeholder="Select Swarm Service"
                          />
                          <SwarmServiceSourcePreview
                            preview={swarmServicePreview.data?.data}
                            isLoading={swarmServicePreview.isLoading || swarmServicePreview.isFetching}
                            hasSelection={Boolean(currentSwarmServiceId)}
                          />
                        </div>
                      ),
                    }),
                  ]
                : []),
            ],
          }),
          defineGroupField<BackupPolicyFormValue>({
            id: 'destination',
            label: 'Destination',
            items: [
              defineField<BackupPolicyFormValue, 'backupRepositoryId'>({
                key: 'backupRepositoryId',
                label: 'Repository',
                required: true,
                description: 'Backup repository where snapshots are stored.',
                validate: (value) => (!value ? 'Backup repository is required' : repositoryCompatibilityMessage),
                render: (value, set) => (
                  <div className="flex max-w-150 flex-col gap-2">
                    <ResourceSelectorField
                      targetType={LookupResourceType.BackupRepository}
                      selected={value}
                      disabled={disabled || isSourceLocked(resource)}
                      onSelect={(repository: { id: string } | undefined) =>
                        set({ backupRepositoryId: repository?.id ?? '' })
                      }
                      placeholder="Select Backup Repository"
                    />
                    {repositoryCompatibilityMessage && (
                      <div className="flex items-start gap-2 rounded-sm border border-amber-500/30 bg-amber-500/10 p-3 text-xs text-amber-700 dark:text-amber-300">
                        <TriangleAlert className="mt-0.5 size-3.5 shrink-0" />
                        <span>{repositoryCompatibilityMessage}</span>
                      </div>
                    )}
                  </div>
                ),
              }),
              defineField<BackupPolicyFormValue, 'keepLastSuccessful'>({
                key: 'keepLastSuccessful',
                label: 'Retention',
                required: true,
                description: 'Number of successful snapshots to keep for this policy.',
                validate: (value) => {
                  const n = Number(value);
                  if (!Number.isFinite(n) || n < 1) return 'Retention must be at least 1';
                  return null;
                },
                render: (value, set) => (
                  <FieldInput
                    type="number"
                    value={value ?? 14}
                    disabled={disabled}
                    onChange={(keepLastSuccessful) => set({ keepLastSuccessful })}
                  />
                ),
              }),
            ],
          }),
        ],
      }),
      Execution: defineSection<BackupPolicyFormValue>({
        title: 'Execution',
        items: [
          defineGroupField<BackupPolicyFormValue>({
            id: 'runtime',
            label: 'Runtime',
            items: [
              defineField<BackupPolicyFormValue, 'enabled'>({
                key: 'enabled',
                label: 'Enabled',
                description: 'Disabled policies can be saved but cannot run on schedule.',
                render: (value, set) => (
                  <FieldSwitch
                    checked={value ?? true}
                    id="backup-policy-enabled"
                    disabled={disabled}
                    onChange={(enabled) => set({ enabled })}
                  />
                ),
              }),
              defineField<BackupPolicyFormValue, 'runAsActorId'>({
                key: 'runAsActorId',
                label: 'Run as',
                description: 'Permissions are evaluated at run time for this identity.',
                render: (value, set) => (
                  <ResourceSelectorField
                    targetType={LookupResourceType.RunAsActor}
                    selected={value ?? undefined}
                    placeholder="Current user"
                    disabled={disabled}
                    onSelect={(actor) => set({ runAsActorId: actor?.id ?? null })}
                  />
                ),
              }),
              defineField<BackupPolicyFormValue, 'timeoutSeconds'>({
                key: 'timeoutSeconds',
                label: 'Timeout',
                required: true,
                description: 'Maximum backup duration in seconds.',
                validate: (value) => {
                  const n = Number(value);
                  if (!Number.isFinite(n) || n < 1) return 'Timeout must be at least 1 second';
                  return null;
                },
                render: (value, set) => (
                  <FieldInput
                    type="number"
                    value={value ?? 1800}
                    disabled={disabled}
                    onChange={(timeoutSeconds) => set({ timeoutSeconds })}
                  />
                ),
              }),
              defineField<BackupPolicyFormValue, 'alertOnFailure'>({
                key: 'alertOnFailure',
                label: 'Alert On Failure',
                description: 'Raise an alert-worthy event when the backup fails or times out.',
                render: (value, set) => (
                  <FieldSwitch
                    checked={value ?? true}
                    id="backup-policy-alert-on-failure"
                    disabled={disabled}
                    onChange={(alertOnFailure) => set({ alertOnFailure })}
                  />
                ),
              }),
            ],
          }),
        ],
      }),
      Schedule: defineSection<BackupPolicyFormValue>({
        title: 'Schedule',
        items: [
          defineGroupField<BackupPolicyFormValue>({
            id: 'schedule',
            label: 'Schedule',
            description: 'Run this policy automatically from a cron expression.',
            requiredLicense: automatedOperationsEnabled ? undefined : 'Team',
            items: [
              defineField<BackupPolicyFormValue, 'scheduleEnabled'>({
                key: 'scheduleEnabled',
                label: 'Enabled',
                render: (value, set) => (
                  <FieldSwitch
                    checked={value ?? false}
                    id="backup-policy-schedule-enabled"
                    disabled={disabled || (!value && !automatedOperationsEnabled)}
                    onChange={(scheduleEnabled) => set({ scheduleEnabled })}
                  />
                ),
              }),
              defineField<BackupPolicyFormValue, 'cron'>({
                key: 'cron',
                label: 'Cron',
                required: currentScheduleEnabled,
                disabled: !currentScheduleEnabled,
                validate: (value) =>
                  currentScheduleEnabled && !String(value ?? '').trim()
                    ? 'Cron is required when schedule is enabled'
                    : null,
                render: (value, set) => (
                  <FieldInput
                    value={value ?? ''}
                    placeholder="0 2 * * *"
                    disabled={disabled || !currentScheduleEnabled || !automatedOperationsEnabled}
                    onChange={(cron) => set({ cron })}
                  />
                ),
              }),
              defineField<BackupPolicyFormValue, 'timeZone'>({
                key: 'timeZone',
                label: 'Time Zone',
                required: currentScheduleEnabled,
                disabled: !currentScheduleEnabled,
                render: (value, set) => (
                  <TimezoneSelectField
                    value={value ?? 'UTC'}
                    disabled={disabled || !currentScheduleEnabled || !automatedOperationsEnabled}
                    onChange={(timeZone) => set({ timeZone })}
                    className="w-100"
                  />
                ),
              }),
            ],
          }),
          defineGroupField<BackupPolicyFormValue>({
            id: 'webhook',
            label: 'Webhook',
            description: 'Allow a webhook to queue this backup policy through the shared listener.',
            requiredLicense: automatedOperationsEnabled ? undefined : 'Team',
            items: [
              defineField<BackupPolicyFormValue, 'webhook'>({
                key: 'webhook',
                label: 'Enabled',
                render: (value, set) => (
                  <WebhookConfigField
                    resourceType="backup-policy"
                    resourceId={id}
                    execution="run"
                    value={value ?? { enabled: false }}
                    disabled={disabled}
                    enableDisabled={!automatedOperationsEnabled}
                    onChange={(webhook) => set({ webhook: webhook as BackupWebhookConfig })}
                  />
                ),
              }),
            ],
          }),
        ],
      }),
    }),
    [
      currentPlatformId,
      currentDeploymentId,
      currentSwarmServiceId,
      currentScheduleEnabled,
      currentSource,
      automatedOperationsEnabled,
      currentStackId,
      currentSourceType,
      deploymentPreview.data?.data,
      deploymentPreview.isFetching,
      deploymentPreview.isLoading,
      disabled,
      id,
      mode,
      original.source,
      isSwarmSourcePlatform,
      repositoryCompatibilityMessage,
      resource,
      stackPreview.data?.data,
      stackPreview.isFetching,
      stackPreview.isLoading,
      swarmServicePreview.data?.data,
      swarmServicePreview.isFetching,
      swarmServicePreview.isLoading,
      volumeList.data?.data,
      volumeList.isFetching,
      volumeList.isLoading,
    ],
  );

  return (
    <FormShell
      schema={schema}
      original={original}
      update={update}
      setUpdate={setUpdate}
      onSave={handleSave}
      disabled={disabled}
      mode={mode}
      pending={isPending}
      draftKey={`backup-policy:${id ?? 'new'}`}
      draftVersion={resource?.rowVersion ?? 1}
    />
  );
}

function VolumeSelector({
  value,
  dockerNodeId,
  volumes,
  isLoading,
  hasPlatform,
  disabled,
  isSwarm,
  onChange,
}: {
  value?: string;
  dockerNodeId?: string | null;
  volumes: VolumeView[];
  isLoading: boolean;
  hasPlatform: boolean;
  disabled?: boolean;
  isSwarm: boolean;
  onChange: (volume?: VolumeView) => void;
}) {
  const selectedId = volumeSelectorId(value ?? '', dockerNodeId);
  const options = useMemo(() => {
    const selectableVolumes = isSwarm
      ? volumes.filter(
          (volume) =>
            Boolean(volume.dockerNodeId) &&
            !volume.isStale &&
            volume.driver.toLowerCase() === 'local' &&
            !volume.clusterVolume &&
            Object.keys(volume.options ?? {}).length === 0,
        )
      : volumes;
    const items: VolumeSelectorItem[] = selectableVolumes.map((volume) => ({
      ...volume,
      id: volumeSelectorId(volume.name, volume.dockerNodeId),
      description: formatVolumeDescription(volume),
    }));

    if (value && !items.some((item) => item.id === selectedId)) {
      items.push({
        id: selectedId,
        name: value,
        dockerNodeId: dockerNodeId ?? null,
        description: 'Saved volume name',
      } as VolumeSelectorItem);
    }

    return items;
  }, [dockerNodeId, isSwarm, selectedId, value, volumes]);

  if (!hasPlatform) {
    return <div className="text-xs text-muted-foreground">Select a platform to load Docker volumes.</div>;
  }

  if (isLoading) {
    return (
      <div className="inline-flex items-center gap-2 text-xs text-muted-foreground">
        <LoaderCircle className="size-3.5 animate-spin" />
        Loading Docker volumes...
      </div>
    );
  }

  return (
    <div className="flex max-w-150 flex-col gap-2">
      {options.length > 0 || value ? (
        <ResourceSelectorField<VolumeSelectorItem>
          targetType={LookupResourceType.Volume}
          selected={selectedId}
          items={options}
          queryEnabled={false}
          allowClear={false}
          disabled={disabled}
          searchPlaceholder="Search volumes..."
          placeholder="Select a volume"
          onSelect={onChange}
          renderItem={(volume) => (
            <div className="flex min-w-0 flex-col">
              <span className="break-all font-medium leading-5">{volume.name}</span>
              <span className="text-xs text-muted-foreground">{volume.description}</span>
            </div>
          )}
        />
      ) : (
        <div className="rounded-sm border border-dashed bg-muted/20 p-3 text-xs text-muted-foreground">
          No Docker named volumes were found on this platform.
        </div>
      )}
      <p className="text-xs text-muted-foreground">
        Only Docker named volumes can be backed up. Bind mounts such as ./data:/app/data or /host/path:/data are not
        Docker volumes and will not appear here.
      </p>
    </div>
  );
}

type VolumeSelectorItem = {
  id: string;
  description: string;
} & VolumeView;

function volumeSelectorId(volumeName: string, dockerNodeId?: string | null) {
  return dockerNodeId ? `${dockerNodeId}:${volumeName}` : volumeName;
}

function formatVolumeDescription(volume: VolumeView) {
  return [
    volume.driver || 'local',
    volume.inUse ? 'In use' : 'Not in use',
    volume.backupCoverage ? 'Protected' : 'Not protected',
    volume.nodeHostname ?? volume.dockerNodeId,
    formatVolumeSize(volume.usageData?.size),
  ].join(' · ');
}

function formatVolumeSize(value: number | string | null | undefined) {
  if (value == null) return 'Size unavailable';

  const bytes = Number(value);
  return Number.isFinite(bytes) && bytes >= 0 ? byteTransform(bytes, 2) : 'Size unavailable';
}

function StackSourcePreview({
  preview,
  isLoading,
  hasSelection,
}: {
  preview?: StackBackupSourcePreviewView;
  isLoading: boolean;
  hasSelection: boolean;
}) {
  if (!hasSelection) {
    return <div className="text-xs text-muted-foreground">Select a stack to preview resolved volumes.</div>;
  }

  if (isLoading) {
    return (
      <div className="inline-flex items-center gap-2 text-xs text-muted-foreground">
        <LoaderCircle className="size-3.5 animate-spin" />
        Loading stack volumes...
      </div>
    );
  }

  if (!preview) return null;

  return (
    <div className="rounded-sm border border-border/70 bg-muted/20 p-3">
      <div className="flex min-w-0 flex-col gap-3">
        <div className="flex min-w-0 flex-wrap items-center gap-2 text-sm">
          <Layers className="size-3.5 text-muted-foreground" />
          <span className="font-medium">{preview.stackName}</span>
          <span className="text-muted-foreground">on</span>
          <span className="truncate text-muted-foreground">{preview.platformName}</span>
        </div>

        {preview.volumes.length > 0 ? (
          <div className="flex flex-wrap gap-1.5">
            {preview.volumes.map((volume) => (
              <div
                key={`${volume.dockerNodeId ?? ''}:${volume.name}`}
                className="inline-flex min-w-0 max-w-full items-center gap-1.5 rounded-sm border bg-background px-2 py-1 text-xs">
                <Database className="size-3 shrink-0 text-muted-foreground" />
                <span className="truncate" title={volume.name}>
                  {volume.name}
                </span>
                {volume.nodeHostname && (
                  <span className="truncate text-muted-foreground" title={volume.nodeHostname}>
                    {volume.nodeHostname}
                  </span>
                )}
                <Badge variant="secondary" className="h-5 rounded-sm px-1.5 text-[10px]">
                  {volumeKindLabel(volume.kind)}
                </Badge>
                {volume.isShared && (
                  <Badge variant="outline" className="h-5 rounded-sm px-1.5 text-[10px]">
                    Shared
                  </Badge>
                )}
                {volume.hasBackupCoverage && (
                  <Badge className="h-5 rounded-sm bg-green-500/15 px-1.5 text-[10px] text-green-700 dark:text-green-300">
                    Protected
                  </Badge>
                )}
              </div>
            ))}
          </div>
        ) : (
          <div className="text-xs text-muted-foreground">No named Docker volumes were resolved for this stack.</div>
        )}

        {preview.warnings.length > 0 && (
          <div className="flex flex-col gap-1.5 border-t border-dashed pt-3">
            {preview.warnings.map((warning) => (
              <div key={warning} className="flex items-start gap-2 text-xs text-amber-700 dark:text-amber-300">
                <TriangleAlert className="mt-0.5 size-3.5 shrink-0" />
                <span>{warning}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function DeploymentSourcePreview({
  preview,
  isLoading,
  hasSelection,
}: {
  preview?: DeploymentBackupSourcePreviewView;
  isLoading: boolean;
  hasSelection: boolean;
}) {
  if (!hasSelection) {
    return <div className="text-xs text-muted-foreground">Select a deployment to preview resolved volumes.</div>;
  }

  if (isLoading) {
    return (
      <div className="inline-flex items-center gap-2 text-xs text-muted-foreground">
        <LoaderCircle className="size-3.5 animate-spin" />
        Loading deployment volumes...
      </div>
    );
  }

  if (!preview) return null;

  return (
    <div className="rounded-sm border border-border/70 bg-muted/20 p-3">
      <div className="flex min-w-0 flex-col gap-3">
        <div className="flex min-w-0 flex-wrap items-center gap-2 text-sm">
          <Box className="size-3.5 text-muted-foreground" />
          <span className="font-medium">{preview.deploymentName}</span>
          <span className="text-muted-foreground">on</span>
          <span className="truncate text-muted-foreground">{preview.platformName}</span>
        </div>

        {preview.volumes.length > 0 ? (
          <div className="flex flex-wrap gap-1.5">
            {preview.volumes.map((volume) => (
              <div
                key={`${volume.dockerNodeId ?? ''}:${volume.name}`}
                className="inline-flex min-w-0 max-w-full items-center gap-1.5 rounded-sm border bg-background px-2 py-1 text-xs">
                <Database className="size-3 shrink-0 text-muted-foreground" />
                <span className="truncate" title={volume.name}>
                  {volume.name}
                </span>
                {volume.nodeHostname && (
                  <span className="truncate text-muted-foreground" title={volume.nodeHostname}>
                    {volume.nodeHostname}
                  </span>
                )}
                <Badge variant="secondary" className="h-5 rounded-sm px-1.5 text-[10px]">
                  {volumeKindLabel(volume.kind)}
                </Badge>
                {volume.hasBackupCoverage && (
                  <Badge className="h-5 rounded-sm bg-green-500/15 px-1.5 text-[10px] text-green-700 dark:text-green-300">
                    Protected
                  </Badge>
                )}
              </div>
            ))}
          </div>
        ) : (
          <div className="text-xs text-muted-foreground">
            No named Docker volumes were resolved for this deployment.
          </div>
        )}

        {preview.warnings.length > 0 && (
          <div className="flex flex-col gap-1.5 border-t border-dashed pt-3">
            {preview.warnings.map((warning) => (
              <div key={warning} className="flex items-start gap-2 text-xs text-amber-700 dark:text-amber-300">
                <TriangleAlert className="mt-0.5 size-3.5 shrink-0" />
                <span>{warning}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function SwarmServiceSourcePreview({
  preview,
  isLoading,
  hasSelection,
}: {
  preview?: SwarmServiceBackupSourcePreviewView;
  isLoading: boolean;
  hasSelection: boolean;
}) {
  if (!hasSelection) {
    return <div className="text-xs text-muted-foreground">Select a Swarm Service to preview its task Volumes.</div>;
  }

  if (isLoading) {
    return (
      <div className="inline-flex items-center gap-2 text-xs text-muted-foreground">
        <LoaderCircle className="size-3.5 animate-spin" />
        Loading Swarm Service Volumes...
      </div>
    );
  }

  if (!preview) return null;

  return (
    <div className="rounded-sm border border-border/70 bg-muted/20 p-3">
      <div className="flex min-w-0 flex-col gap-3">
        <div className="flex min-w-0 flex-wrap items-center gap-2 text-sm">
          <Layers className="size-3.5 text-muted-foreground" />
          <span className="font-medium">{preview.swarmServiceName}</span>
          <span className="text-muted-foreground">on</span>
          <span className="truncate text-muted-foreground">{preview.platformName}</span>
        </div>

        {preview.volumes.length > 0 ? (
          <div className="flex flex-wrap gap-1.5">
            {preview.volumes.map((volume) => (
              <div
                key={`${volume.dockerNodeId ?? ''}:${volume.name}`}
                className="inline-flex min-w-0 max-w-full items-center gap-1.5 rounded-sm border bg-background px-2 py-1 text-xs">
                <Database className="size-3 shrink-0 text-muted-foreground" />
                <span className="truncate" title={volume.name}>
                  {volume.name}
                </span>
                <span
                  className="truncate text-muted-foreground"
                  title={volume.nodeHostname ?? volume.dockerNodeId ?? ''}>
                  {volume.nodeHostname ?? volume.dockerNodeId}
                </span>
                {volume.isShared && (
                  <Badge variant="outline" className="h-5 rounded-sm px-1.5 text-[10px]">
                    Shared
                  </Badge>
                )}
                {volume.hasBackupCoverage && (
                  <Badge className="h-5 rounded-sm bg-green-500/15 px-1.5 text-[10px] text-green-700 dark:text-green-300">
                    Protected
                  </Badge>
                )}
              </div>
            ))}
          </div>
        ) : (
          <div className="text-xs text-muted-foreground">No supported local named Volumes were resolved.</div>
        )}

        {preview.warnings.length > 0 && (
          <div className="flex flex-col gap-1.5 border-t border-dashed pt-3">
            {preview.warnings.map((warning) => (
              <div key={warning} className="flex items-start gap-2 text-xs text-amber-700 dark:text-amber-300">
                <TriangleAlert className="mt-0.5 size-3.5 shrink-0" />
                <span>{warning}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function volumeKindLabel(kind: StackVolumeKind) {
  switch (kind) {
    case StackVolumeKind.DeclaredNamed:
      return 'Declared';
    case StackVolumeKind.ExternalNamed:
      return 'External';
    case StackVolumeKind.AnonymousNamed:
      return 'Runtime';
    default:
      return kind;
  }
}

function createCitadelSystemSource(): Extract<BackupSourceSpec, { $type: 'CitadelSystem' }> {
  return {
    $type: BackupSourceType.CitadelSystem,
    stableKey: null,
  };
}

function createDockerVolumeSource(): Extract<BackupSourceSpec, { $type: 'DockerVolume' }> {
  return {
    $type: BackupSourceType.DockerVolume,
    platformId: '',
    volumeName: '',
    consistency: VolumeBackupConsistency.Live,
    stableKey: null,
  };
}

function createStackSource(): Extract<BackupSourceSpec, { $type: 'Stack' }> {
  return {
    $type: BackupSourceType.Stack,
    stackId: '',
    stableKey: null,
  };
}

function createDeploymentSource(): Extract<BackupSourceSpec, { $type: 'Deployment' }> {
  return {
    $type: BackupSourceType.Deployment,
    deploymentId: '',
    stableKey: null,
  };
}

function createSwarmServiceSource(): Extract<BackupSourceSpec, { $type: 'SwarmService' }> {
  return {
    $type: BackupSourceType.SwarmService,
    swarmServiceId: '',
    stableKey: null,
  };
}

function isSourceLocked(resource?: BackupPolicyView) {
  return Boolean(resource?.firstSuccessfulRunAt);
}

function getRepositoryCompatibilityMessage(
  sourceType: BackupSourceSpec['$type'],
  repository?: BackupRepositoryView,
  sourcePlatform?: PlatformView,
  sourcePlatformId?: string,
) {
  if (!repository || repository.type !== BackupRepositoryType.FileSystem) return null;

  const spec = repository.spec as BackupRepositorySpecFileSystemBackupRepositorySpec;
  if (sourceType === BackupSourceType.CitadelSystem) {
    return spec.location === BackupExecutionLocation.Core
      ? null
      : 'Citadel backups can only use a Core filesystem repository or an S3-compatible repository.';
  }

  if (!sourcePlatformId || !sourcePlatform) return null;

  if (sourcePlatform.type === PlatformType.DockerSwarm) {
    return 'Docker Swarm Volume backups require an S3-compatible repository.';
  }

  if (spec.location === BackupExecutionLocation.Core && sourcePlatform.connectorType !== PlatformConnectorType.Local) {
    return 'Core filesystem repositories cannot back up Docker volumes on regular or edge agents. Use an S3-compatible repository or a filesystem repository on the same platform.';
  }

  if (spec.location === BackupExecutionLocation.Platform && spec.platformId !== sourcePlatform.id) {
    return `This filesystem repository belongs to another platform. Select a repository on ${sourcePlatform.name} or use an S3-compatible repository.`;
  }

  return null;
}

function toFormValue(resource?: BackupPolicyView): BackupPolicyFormValue {
  if (!resource) return emptyPolicy();

  return {
    id: resource.id,
    name: resource.name,
    description: resource.description ?? '',
    source: resource.source,
    backupRepositoryId: resource.backupRepositoryId,
    enabled: resource.enabled,
    scheduleEnabled: Boolean(resource.cron),
    cron: resource.cron ?? '',
    timeZone: resource.timeZone ?? 'UTC',
    webhook: resource.webhook ?? { enabled: false },
    keepLastSuccessful: resource.keepLastSuccessful,
    timeoutSeconds: resource.timeoutSeconds,
    alertOnFailure: resource.alertOnFailure,
    runAsActorId: resource.runAsActorId,
    tagIds: resource.tags?.map((tag) => tag.id) ?? [],
  };
}

function mergeSource(original: BackupSourceSpec, update?: BackupSourceSpec | null): BackupSourceSpec {
  if (!update) return original;
  if (update.$type && update.$type !== original.$type) return update;
  return { ...original, ...update } as BackupSourceSpec;
}

function toCreateInput(payload: BackupPolicyFormValue): BackupPolicyInput {
  return {
    name: payload.name,
    description: payload.description ?? null,
    source: normalizeSource(payload.source),
    backupRepositoryId: payload.backupRepositoryId,
    enabled: payload.enabled ?? true,
    cron: payload.scheduleEnabled ? payload.cron || null : null,
    timeZone: payload.scheduleEnabled ? payload.timeZone || 'UTC' : null,
    webhook: normalizeWebhook(payload.webhook),
    keepLastSuccessful: payload.keepLastSuccessful ?? 14,
    timeoutSeconds: payload.timeoutSeconds ?? 1800,
    alertOnFailure: payload.alertOnFailure ?? true,
    runAsActorId: payload.runAsActorId || null,
    tagIds: payload.tagIds ?? [],
  };
}

function toUpdateInput(
  payload: BackupPolicyFormValue,
  update: Partial<BackupPolicyFormValue>,
): UpdateBackupPolicyInput {
  const next: UpdateBackupPolicyInput = {};

  if ('description' in update) next.description = payload.description ?? null;
  if ('source' in update) next.source = normalizeSource(payload.source);
  if ('backupRepositoryId' in update) next.backupRepositoryId = payload.backupRepositoryId;
  if ('enabled' in update) next.enabled = payload.enabled ?? true;
  if ('keepLastSuccessful' in update) next.keepLastSuccessful = payload.keepLastSuccessful ?? 14;
  if ('timeoutSeconds' in update) next.timeoutSeconds = payload.timeoutSeconds ?? 1800;
  if ('alertOnFailure' in update) next.alertOnFailure = payload.alertOnFailure ?? true;
  if ('runAsActorId' in update) next.runAsActorId = payload.runAsActorId || null;
  if ('webhook' in update) next.webhook = normalizeWebhook(payload.webhook);

  if ('scheduleEnabled' in update || 'cron' in update || 'timeZone' in update) {
    next.cron = payload.scheduleEnabled ? payload.cron || null : null;
    next.timeZone = payload.scheduleEnabled ? payload.timeZone || 'UTC' : null;
  }

  return next;
}

function normalizeWebhook(webhook: BackupWebhookConfig | null | undefined): BackupWebhookConfig | null {
  if (!webhook) return null;

  return {
    ...webhook,
    secret: webhook.secret?.trim() || null,
    branchFilter: webhook.branchFilter?.trim() || null,
  };
}

function normalizeSource(source: BackupSourceSpec): BackupSourceSpec {
  if (source.$type === BackupSourceType.DockerVolume) {
    const dockerSource = source as BackupSourceSpecDockerVolumeBackupSource;
    return {
      $type: BackupSourceType.DockerVolume,
      platformId: dockerSource.platformId,
      volumeName: dockerSource.volumeName,
      consistency: dockerSource.consistency ?? VolumeBackupConsistency.Live,
      dockerNodeId: dockerSource.dockerNodeId ?? null,
      stableKey: dockerSource.stableKey ?? null,
    };
  }

  if (source.$type === BackupSourceType.Stack) {
    const stackSource = source as BackupSourceSpecStackBackupSource;
    return {
      $type: BackupSourceType.Stack,
      stackId: stackSource.stackId,
      stableKey: stackSource.stableKey ?? null,
    };
  }

  if (source.$type === BackupSourceType.Deployment) {
    const deploymentSource = source as BackupSourceSpecDeploymentBackupSource;
    return {
      $type: BackupSourceType.Deployment,
      deploymentId: deploymentSource.deploymentId,
      stableKey: deploymentSource.stableKey ?? null,
    };
  }

  if (source.$type === BackupSourceType.SwarmService) {
    const serviceSource = source as BackupSourceSpecSwarmServiceBackupSource;
    return {
      $type: BackupSourceType.SwarmService,
      swarmServiceId: serviceSource.swarmServiceId,
      stableKey: serviceSource.stableKey ?? null,
    };
  }

  return createCitadelSystemSource();
}
