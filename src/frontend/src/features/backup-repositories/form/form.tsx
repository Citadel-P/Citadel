import { type RepositorySecretTarget, useBackupRepositorySecrets } from '../hooks/useBackupRepositorySecrets';
import { useReconcileBackupRepository } from '../cache';
import {
  BackupRepositoryFormInput,
  createDefaultInput,
  createFileSystemSpec,
  createS3Spec,
  mergeSpec,
  toCreateInput,
  toUpdateInput,
} from '../input';
import {
  BackupRepositorySpec,
  BackupExecutionLocation,
  BackupRepositoryStatus,
  BackupRepositoryType,
  BackupRepositoryView,
  PlatformView,
  S3BucketLookup,
} from '@/api/generated/api.types';
import { ResourceSelectorField } from '@/components/custom/common';
import {
  defineField,
  defineGroupField,
  defineSection,
  FieldInput,
  FieldSwitch,
  FormShell,
  ItemSelector,
} from '@/components/custom/form-builder';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { useMutate, useRead, useSaveResource } from '@/lib/hooks';
import { Cloud, FolderLock } from 'lucide-react';
import { useMemo, useState } from 'react';
import { useParams } from 'react-router';

export const repositoryTypes = {
  FileSystem: {
    label: 'Filesystem',
    description: 'Store repository data on Core or a connected Docker host.',
    icon: FolderLock,
  },
  S3Compatible: {
    label: 'S3-compatible',
    description: 'Store repository data in an S3-compatible object store.',
    icon: Cloud,
  },
} as const;

export const bucketLookupOptions = {
  Auto: {
    label: 'Auto',
    description: 'Use Restic default bucket lookup behavior.',
  },
  Path: {
    label: 'Path',
    description: 'Use path-style bucket lookup.',
  },
  Dns: {
    label: 'DNS',
    description: 'Use DNS-style bucket lookup.',
  },
} as const;

export const fileSystemLocationOptions = {
  Core: {
    label: 'Core filesystem',
    description: 'Store snapshots on the Citadel Core host or container volume.',
  },
  Platform: {
    label: 'Platform filesystem',
    description: 'Store snapshots on a Local, Agent, or Edge Agent Docker host.',
  },
} as const;

export function BackupRepositoryForm({
  mode,
  resource,
  disabled,
}: {
  mode: 'add' | 'edit';
  resource?: BackupRepositoryView;
  disabled?: boolean;
}) {
  const id = useParams().id;
  const [update, setUpdate] = useState<Partial<BackupRepositoryFormInput>>({});
  const reconcile = useReconcileBackupRepository();
  const { mutateAsync: createBackupRepository } = useMutate('createBackupRepository', { onSuccess: () => reconcile() });
  const { mutateAsync: updateBackupRepository } = useMutate('updateBackupRepository', {
    onSuccess: (_, { id }) => reconcile(id),
  });

  const { save: handleSave, isPending } = useSaveResource<BackupRepositoryFormInput, any>({
    mode,
    basePath: 'backup-repositories',
    entityName: 'Backup repository',
    onCreate: (payload) => createBackupRepository({ data: toCreateInput(payload) }),
    onUpdate: (payload) => updateBackupRepository({ id: id!, data: toUpdateInput(payload, resource!) } as any),
    onRefresh: () => {
      localStorage.removeItem(`backup-repository:${id ?? 'new'}`);
    },
  });

  const original = resource ?? createDefaultInput();
  const currentSpec = mergeSpec(original.spec, update.spec);
  const selectedType = currentSpec?.$type ?? BackupRepositoryType.FileSystem;
  const fileSystemSpec = currentSpec as Extract<BackupRepositorySpec, { $type: 'FileSystem' }>;
  const selectedLocation = fileSystemSpec.location ?? BackupExecutionLocation.Core;
  const specDisabled = disabled || (mode === 'edit' && resource?.status === BackupRepositoryStatus.Ready);

  const schema = useMemo(
    () => ({
      '': defineSection<BackupRepositoryFormInput>({
        title: '',
        items: [
          defineGroupField<BackupRepositoryFormInput>({
            id: 'details',
            label: 'Details',
            items: [
              defineField<BackupRepositoryFormInput, 'name'>({
                key: 'name',
                label: 'Name',
                required: true,
                description: 'Unique name for this repository.',
                render: (value, set) => (
                  <FieldInput
                    value={value ?? ''}
                    disabled={disabled}
                    placeholder="daily-core-backups"
                    onChange={(name) => set({ name })}
                  />
                ),
              }),
            ],
          }),
          defineGroupField<BackupRepositoryFormInput>({
            id: 'destination',
            label: 'Destination',
            description:
              mode === 'edit' && resource?.status === BackupRepositoryStatus.Ready
                ? 'Ready repository destinations are locked. Create a new repository to change the destination.'
                : undefined,
            items: [
              defineField<BackupRepositoryFormInput, 'spec.$type'>({
                key: 'spec.$type',
                label: 'Type',
                required: true,
                disabled: specDisabled,
                render: (value, set) => (
                  <RepositoryTypeSelector
                    value={(value as BackupRepositoryType) ?? BackupRepositoryType.FileSystem}
                    disabled={specDisabled}
                    onChange={(nextType) =>
                      set({
                        spec: nextType === BackupRepositoryType.S3Compatible ? createS3Spec() : createFileSystemSpec(),
                      })
                    }
                  />
                ),
              }),
              defineField<BackupRepositoryFormInput, 'passwordSecretId'>({
                key: 'passwordSecretId',
                label: 'Password Secret',
                required: mode === 'add',
                disabled: mode === 'edit',
                description: 'Stored secret used as the Restic repository password.',
                render: (value, set) => (
                  <SecretSelector
                    value={value ?? undefined}
                    disabled={mode === 'edit'}
                    repository={resource}
                    onChange={(passwordSecretId) => set({ passwordSecretId: passwordSecretId ?? '' })}
                  />
                ),
              }),
              ...(selectedType === BackupRepositoryType.FileSystem
                ? [
                    defineField<BackupRepositoryFormInput, 'spec.location'>({
                      key: 'spec.location',
                      label: 'Location',
                      required: true,
                      disabled: specDisabled,
                      description: 'Choose where this filesystem repository is physically stored.',
                      render: (value, set) => (
                        <ItemSelector
                          collection={fileSystemLocationOptions}
                          value={value ?? BackupExecutionLocation.Core}
                          disabled={specDisabled}
                          onChange={(location: BackupExecutionLocation) =>
                            set({
                              spec: {
                                location,
                                platformId:
                                  location === BackupExecutionLocation.Platform ? fileSystemSpec.platformId : null,
                              } as any,
                            })
                          }
                        />
                      ),
                    }),
                    ...(selectedLocation === BackupExecutionLocation.Platform
                      ? [
                          defineField<BackupRepositoryFormInput, 'spec.platformId'>({
                            key: 'spec.platformId',
                            label: 'Platform',
                            required: true,
                            disabled: specDisabled,
                            description:
                              'Snapshots stay on this Docker host. Swarm backups require an S3-compatible repository.',
                            validate: (value) =>
                              selectedLocation === BackupExecutionLocation.Platform && !value
                                ? 'Platform is required'
                                : null,
                            render: (value, set) => (
                              <FileSystemPlatformSelector
                                selected={value ?? undefined}
                                disabled={specDisabled}
                                onSelect={(platform: PlatformView | undefined) =>
                                  set({ spec: { platformId: platform?.id ?? null } as any })
                                }
                              />
                            ),
                          }),
                        ]
                      : []),
                    defineField<BackupRepositoryFormInput, 'spec.path'>({
                      key: 'spec.path',
                      label: 'Repository Path',
                      required: true,
                      disabled: specDisabled,
                      description:
                        selectedLocation === BackupExecutionLocation.Core
                          ? 'Core filesystem path. Relative paths are created under the configured Core backup repository root.'
                          : 'Existing absolute directory or Docker volume name on the selected host. Agent paths are not paths inside Core.',
                      render: (value, set) => (
                        <FieldInput
                          value={value ?? ''}
                          disabled={specDisabled}
                          placeholder={
                            selectedLocation === BackupExecutionLocation.Core
                              ? 'daily/core'
                              : '/srv/backups or backup-volume'
                          }
                          onChange={(path) => set({ spec: { path } as any })}
                        />
                      ),
                    }),
                  ]
                : []),
            ],
          }),
          ...(selectedType === BackupRepositoryType.S3Compatible
            ? [
                defineGroupField<BackupRepositoryFormInput>({
                  id: 's3',
                  label: 'S3',
                  items: [
                    defineField<BackupRepositoryFormInput, 'spec.endpoint'>({
                      key: 'spec.endpoint',
                      label: 'Endpoint',
                      required: true,
                      disabled: specDisabled,
                      description: 'S3-compatible endpoint URL.',
                      render: (value, set) => (
                        <FieldInput
                          value={value ?? ''}
                          disabled={specDisabled}
                          placeholder="https://s3.example.com"
                          onChange={(endpoint) => set({ spec: { endpoint } as any })}
                        />
                      ),
                    }),
                    defineField<BackupRepositoryFormInput, 'spec.bucket'>({
                      key: 'spec.bucket',
                      label: 'Bucket',
                      required: true,
                      disabled: specDisabled,
                      render: (value, set) => (
                        <FieldInput
                          value={value ?? ''}
                          disabled={specDisabled}
                          placeholder="citadel-backups"
                          onChange={(bucket) => set({ spec: { bucket } as any })}
                        />
                      ),
                    }),
                    defineField<BackupRepositoryFormInput, 'spec.prefix'>({
                      key: 'spec.prefix',
                      label: 'Prefix',
                      required: false,
                      disabled: specDisabled,
                      render: (value, set) => (
                        <FieldInput
                          value={value ?? ''}
                          disabled={specDisabled}
                          placeholder="production/core"
                          onChange={(prefix) => set({ spec: { prefix: prefix || null } as any })}
                        />
                      ),
                    }),
                    defineField<BackupRepositoryFormInput, 'spec.region'>({
                      key: 'spec.region',
                      label: 'Region',
                      required: false,
                      disabled: specDisabled,
                      render: (value, set) => (
                        <FieldInput
                          value={value ?? ''}
                          disabled={specDisabled}
                          placeholder="us-east-1"
                          onChange={(region) => set({ spec: { region: region || null } as any })}
                        />
                      ),
                    }),
                    defineField<BackupRepositoryFormInput, 'spec.bucketLookup'>({
                      key: 'spec.bucketLookup',
                      label: 'Bucket Lookup',
                      required: true,
                      disabled: specDisabled,
                      render: (value, set) => (
                        <ItemSelector
                          collection={bucketLookupOptions}
                          value={value ?? S3BucketLookup.Auto}
                          onChange={(bucketLookup: S3BucketLookup) => set({ spec: { bucketLookup } as any })}
                        />
                      ),
                    }),
                    defineField<BackupRepositoryFormInput, 'spec.accessKeySecretId'>({
                      key: 'spec.accessKeySecretId',
                      label: 'Access Key Secret',
                      required: true,
                      disabled: specDisabled,
                      render: (value, set) => (
                        <SecretSelector
                          value={value}
                          disabled={specDisabled}
                          repository={resource}
                          onChange={(accessKeySecretId) => set({ spec: { accessKeySecretId } as any })}
                        />
                      ),
                    }),
                    defineField<BackupRepositoryFormInput, 'spec.secretKeySecretId'>({
                      key: 'spec.secretKeySecretId',
                      label: 'Secret Key Secret',
                      required: true,
                      disabled: specDisabled,
                      render: (value, set) => (
                        <SecretSelector
                          value={value}
                          disabled={specDisabled}
                          repository={resource}
                          onChange={(secretKeySecretId) => set({ spec: { secretKeySecretId } as any })}
                        />
                      ),
                    }),
                    defineField<BackupRepositoryFormInput, 'spec.sessionTokenSecretId'>({
                      key: 'spec.sessionTokenSecretId',
                      label: 'Session Token Secret',
                      required: false,
                      disabled: specDisabled,
                      render: (value, set) => (
                        <SecretSelector
                          value={value}
                          allowNone
                          disabled={specDisabled}
                          repository={resource}
                          onChange={(sessionTokenSecretId) => set({ spec: { sessionTokenSecretId } as any })}
                        />
                      ),
                    }),
                    defineField<BackupRepositoryFormInput, 'spec.allowInsecureHttp'>({
                      key: 'spec.allowInsecureHttp',
                      label: 'Allow Insecure HTTP',
                      required: false,
                      disabled: specDisabled,
                      description: 'Allow plain HTTP endpoints for local object-store testing.',
                      render: (value, set) => (
                        <FieldSwitch
                          id="backup-repository-allow-insecure-http"
                          checked={value ?? false}
                          onChange={(allowInsecureHttp) => set({ spec: { allowInsecureHttp } as any })}
                        />
                      ),
                    }),
                  ],
                }),
              ]
            : []),
        ],
      }),
    }),
    [disabled, fileSystemSpec.platformId, mode, resource, selectedLocation, selectedType, specDisabled],
  );

  return (
    <FormShell
      mode={mode}
      schema={schema}
      original={original}
      update={update}
      setUpdate={setUpdate}
      onSave={handleSave}
      pending={isPending}
      disabled={disabled}
      draftKey={`backup-repository:${id ?? 'new'}`}
      draftVersion={1}
    />
  );
}

export function RepositoryTypeSelector({
  value,
  onChange,
  disabled,
}: {
  value: BackupRepositoryType;
  onChange: (value: BackupRepositoryType) => void;
  disabled?: boolean;
}) {
  const selected = repositoryTypes[value as keyof typeof repositoryTypes];

  return (
    <Select value={value} onValueChange={(next) => onChange(next as BackupRepositoryType)} disabled={disabled}>
      <SelectTrigger className="w-full max-w-100">
        <SelectValue>
          {selected ? (
            <div className="flex items-center gap-2">
              <selected.icon className="size-4" />
              <span>{selected.label}</span>
            </div>
          ) : (
            'Select repository type'
          )}
        </SelectValue>
      </SelectTrigger>
      <SelectContent className="bg-background">
        {Object.entries(repositoryTypes).map(([key, info]) => (
          <SelectItem key={key} value={key}>
            <div className="flex items-center gap-2">
              <info.icon className="size-4" />
              <div className="flex flex-col">
                <span className="font-medium">{info.label}</span>
                <span className="text-xs text-muted-foreground">{info.description}</span>
              </div>
            </div>
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

export function FileSystemPlatformSelector({
  selected,
  disabled,
  onSelect,
}: {
  selected?: string;
  disabled?: boolean;
  onSelect: (platform: PlatformView | undefined) => void;
}) {
  const { data, isLoading } = useRead('listPlatforms');
  const platforms = data?.data.platforms ?? [];
  return (
    <ResourceSelectorField
      items={platforms.filter((platform) => platform.platformDescriptor?.$type === 'Docker')}
      selected={platforms.find((platform) => platform.id === selected) ?? selected}
      disabled={disabled || isLoading}
      onSelect={onSelect}
      placeholder={isLoading ? 'Loading platforms...' : 'Select Platform'}
      emptyMessage="No standalone Docker platforms available."
    />
  );
}

export function SecretSelector({
  value,
  onChange,
  disabled,
  allowNone = false,
  repository,
}: {
  value?: string | null;
  onChange: (value: string | null) => void;
  disabled?: boolean;
  allowNone?: boolean;
  repository?: RepositorySecretTarget | null;
}) {
  const { canList, secrets, isLoading } = useBackupRepositorySecrets(repository);
  if (!canList) {
    return <span className="text-sm text-muted-foreground">{value ? 'Configured' : 'None'}</span>;
  }

  return (
    <ResourceSelectorField
      items={secrets}
      selected={value ? (secrets.find((secret) => secret.id === value) ?? { id: value, name: value }) : undefined}
      allowClear={allowNone}
      clearLabel="No session token"
      disabled={disabled || isLoading}
      placeholder={isLoading ? 'Loading secrets...' : allowNone ? 'No session token' : 'Select a stored secret'}
      searchPlaceholder="Search secrets..."
      emptyMessage={
        secrets.length === 0 ? 'No stored secrets found. Create one from Bindings first.' : 'No results found.'
      }
      onSelect={(secret) => onChange(secret?.id ?? null)}
    />
  );
}
