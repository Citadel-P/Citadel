import {
  BackupPolicyInput,
  BackupPolicyView,
  BackupSourceSpec,
  BackupSourceSpecCitadelSystemBackupSource,
  BackupSourceSpecDockerVolumeBackupSource,
  BackupSourceSpecStackBackupSource,
  BackupSourceType,
  LookupResourceType,
  PlatformView,
  StackBackupSourcePreviewView,
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
import { Badge } from '@/components/ui/badge';
import { ResourceTagSelector } from '@/features/tags/components';
import { useMutate, useRead, useSaveResource } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { Database, Layers, LoaderCircle, TriangleAlert } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useParams } from 'react-router';

type BackupPolicyFormValue = Omit<BackupPolicyInput, 'runAsActorId'> & {
  id?: string;
  runAsActorId?: string | null;
  scheduleEnabled: boolean;
  tagIds?: string[] | null;
};

const sourceTypes = {
  CitadelSystem: {
    label: 'Citadel system',
    description: 'Back up Citadel database, configuration, keys, and file data.',
  },
  DockerVolume: {
    label: 'Docker volume',
    description: 'Back up one Docker volume from a selected platform.',
  },
  Stack: {
    label: 'Stack',
    description: 'Back up all resolved named volumes used by a stack.',
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

  const original = useMemo(() => toFormValue(resource), [resource]);
  const currentSource = mergeSource(original.source, update.source);
  const currentSourceType = currentSource.$type ?? BackupSourceType.CitadelSystem;
  const currentScheduleEnabled = update.scheduleEnabled ?? original.scheduleEnabled;
  const currentPlatformId =
    currentSource.$type === BackupSourceType.DockerVolume
      ? (currentSource as BackupSourceSpecDockerVolumeBackupSource).platformId
      : undefined;
  const currentStackId =
    currentSource.$type === BackupSourceType.Stack
      ? (currentSource as BackupSourceSpecStackBackupSource).stackId
      : undefined;
  const stackPreviewArgs = useMemo(() => ({ stackId: currentStackId ?? '' }), [currentStackId]);
  const stackPreview = useRead('getStackBackupSourcePreview', stackPreviewArgs, {
    enabled: currentSourceType === BackupSourceType.Stack && Boolean(currentStackId),
  });

  const refreshData = useCallback(() => {
    localStorage.removeItem(`backup-policy:${id ?? 'new'}`);
    queryClient.invalidateQueries({ queryKey: ['listBackupPolicies'] });
    queryClient.invalidateQueries({ queryKey: ['listBackupRuns'] });
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
                              : createCitadelSystemSource(),
                      })
                    }
                  />
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
                        <FieldInput
                          value={value ?? ''}
                          placeholder="postgres_data"
                          disabled={disabled || isSourceLocked(resource) || !currentPlatformId}
                          onChange={(volumeName) => set({ source: { volumeName } as any })}
                        />
                      ),
                    }),
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
                          onChange={(consistency: VolumeBackupConsistency) => set({ source: { consistency } as any })}
                        />
                      ),
                    }),
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
                validate: (value) => (!value ? 'Backup repository is required' : null),
                render: (value, set) => (
                  <ResourceSelectorField
                    targetType={LookupResourceType.BackupRepository}
                    selected={value}
                    disabled={disabled || isSourceLocked(resource)}
                    onSelect={(repository: { id: string } | undefined) =>
                      set({ backupRepositoryId: repository?.id ?? '' })
                    }
                    placeholder="Select Backup Repository"
                  />
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
                label: 'Run As User',
                description: 'Permissions are evaluated at run time for this user.',
                render: (value, set) => (
                  <ResourceSelectorField
                    targetType={LookupResourceType.UserActor}
                    selected={value ?? undefined}
                    placeholder="Current user"
                    disabled={disabled}
                    onSelect={(user) => set({ runAsActorId: user?.id ?? null })}
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
            items: [
              defineField<BackupPolicyFormValue, 'scheduleEnabled'>({
                key: 'scheduleEnabled',
                label: 'Enabled',
                render: (value, set) => (
                  <FieldSwitch
                    checked={value ?? false}
                    id="backup-policy-schedule-enabled"
                    disabled={disabled}
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
                    disabled={disabled || !currentScheduleEnabled}
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
                    disabled={disabled || !currentScheduleEnabled}
                    onChange={(timeZone) => set({ timeZone })}
                    className="w-100"
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
      currentScheduleEnabled,
      currentStackId,
      currentSourceType,
      disabled,
      mode,
      original.source,
      resource,
      stackPreview.data?.data,
      stackPreview.isFetching,
      stackPreview.isLoading,
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
                key={volume.name}
                className="inline-flex min-w-0 max-w-full items-center gap-1.5 rounded-sm border bg-background px-2 py-1 text-xs">
                <Database className="size-3 shrink-0 text-muted-foreground" />
                <span className="truncate" title={volume.name}>
                  {volume.name}
                </span>
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

function createCitadelSystemSource(): BackupSourceSpecCitadelSystemBackupSource {
  return {
    $type: BackupSourceType.CitadelSystem,
    stableKey: null,
  };
}

function createDockerVolumeSource(): BackupSourceSpecDockerVolumeBackupSource {
  return {
    $type: BackupSourceType.DockerVolume,
    platformId: '',
    volumeName: '',
    consistency: VolumeBackupConsistency.Live,
    stableKey: null,
  };
}

function createStackSource(): BackupSourceSpecStackBackupSource {
  return {
    $type: BackupSourceType.Stack,
    stackId: '',
    stableKey: null,
  };
}

function isSourceLocked(resource?: BackupPolicyView) {
  return Boolean(resource?.firstSuccessfulRunAt);
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

  if ('scheduleEnabled' in update || 'cron' in update || 'timeZone' in update) {
    next.cron = payload.scheduleEnabled ? payload.cron || null : null;
    next.timeZone = payload.scheduleEnabled ? payload.timeZone || 'UTC' : null;
  }

  return next;
}

function normalizeSource(source: BackupSourceSpec): BackupSourceSpec {
  if (source.$type === BackupSourceType.DockerVolume) {
    const dockerSource = source as BackupSourceSpecDockerVolumeBackupSource;
    return {
      $type: BackupSourceType.DockerVolume,
      platformId: dockerSource.platformId,
      volumeName: dockerSource.volumeName,
      consistency: dockerSource.consistency ?? VolumeBackupConsistency.Live,
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

  return createCitadelSystemSource();
}
