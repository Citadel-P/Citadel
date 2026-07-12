import {
  BackupExecutionLocation,
  BackupRepositoryInput,
  BackupRepositorySpec,
  BackupRepositorySpecFileSystemBackupRepositorySpec,
  BackupRepositorySpecS3CompatibleBackupRepositorySpec,
  BackupRepositoryStatus,
  BackupRepositoryType,
  BackupRepositoryView,
  S3BucketLookup,
  SecretDefinitionView,
  UpdateBackupRepositoryInput,
} from '@/api/generated/api.types';
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
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { useMutate, useRead, useSaveResource } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { Cloud, FolderLock } from 'lucide-react';
import { useMemo, useState } from 'react';
import { useParams } from 'react-router';

type BackupRepositoryFormInput = BackupRepositoryInput & Partial<BackupRepositoryView>;

const createFileSystemSpec = (): BackupRepositorySpecFileSystemBackupRepositorySpec => ({
  $type: 'FileSystem',
  location: BackupExecutionLocation.Core,
  platformId: null,
  path: '',
});

const createS3Spec = (): BackupRepositorySpecS3CompatibleBackupRepositorySpec => ({
  $type: 'S3Compatible',
  endpoint: '',
  bucket: '',
  prefix: null,
  region: null,
  bucketLookup: S3BucketLookup.Auto,
  accessKeySecretId: '',
  secretKeySecretId: '',
  sessionTokenSecretId: null,
  allowInsecureHttp: false,
});

const createDefaultInput = (): BackupRepositoryFormInput => ({
  name: '',
  description: null,
  passwordSecretId: '',
  spec: createFileSystemSpec(),
});

const repositoryTypes = {
  FileSystem: {
    label: 'Filesystem',
    description: 'Store repository data on a Core-mounted path.',
    icon: FolderLock,
  },
  S3Compatible: {
    label: 'S3-compatible',
    description: 'Store repository data in an S3-compatible object store.',
    icon: Cloud,
  },
} as const;

const bucketLookupOptions = {
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
  const queryClient = useQueryClient();
  const [update, setUpdate] = useState<Partial<BackupRepositoryFormInput>>({});
  const { mutateAsync: createBackupRepository } = useMutate('createBackupRepository');
  const { mutateAsync: updateBackupRepository } = useMutate('updateBackupRepository');

  const { save: handleSave, isPending } = useSaveResource<BackupRepositoryFormInput, any>({
    mode,
    basePath: 'backup-repositories',
    entityName: 'Backup repository',
    onCreate: (payload) => createBackupRepository({ data: toCreateInput(payload) }),
    onUpdate: (payload) => updateBackupRepository({ id: id!, data: toUpdateInput(payload) } as any),
    onRefresh: () => {
      localStorage.removeItem(`backup-repository:${id ?? 'new'}`);
      queryClient.invalidateQueries({ queryKey: ['listBackupRepositories'] });
      if (id) queryClient.invalidateQueries({ queryKey: ['getBackupRepository', { id }] });
    },
  });

  const original = resource ?? createDefaultInput();
  const currentSpec = mergeSpec(original.spec, update.spec);
  const selectedType = currentSpec?.$type ?? BackupRepositoryType.FileSystem;
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
              ...(mode === 'add'
                ? [
                    defineField<BackupRepositoryFormInput, 'name'>({
                      key: 'name',
                      label: 'Name',
                      required: true,
                      description: 'Unique name for this repository.',
                      render: (value, set) => (
                        <FieldInput
                          value={value ?? ''}
                          placeholder="daily-core-backups"
                          onChange={(name) => set({ name })}
                        />
                      ),
                    }),
                  ]
                : []),
              defineField<BackupRepositoryFormInput, 'description'>({
                key: 'description',
                label: 'Description',
                required: false,
                description: 'Optional notes for operators.',
                render: (value, set) => (
                  <FieldTextArea value={value ?? ''} onChange={(description) => set({ description })} />
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
                    value={value}
                    disabled={mode === 'edit'}
                    onChange={(passwordSecretId) => set({ passwordSecretId })}
                  />
                ),
              }),
              ...(selectedType === BackupRepositoryType.FileSystem
                ? [
                    defineField<BackupRepositoryFormInput, 'spec.path'>({
                      key: 'spec.path',
                      label: 'Repository Path',
                      required: true,
                      disabled: specDisabled,
                      description: 'Absolute path mounted into Citadel Core and allowed by backup configuration.',
                      render: (value, set) => (
                        <FieldInput
                          value={value ?? ''}
                          disabled={specDisabled}
                          placeholder="/backups/citadel"
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
    [mode, resource?.status, selectedType, specDisabled],
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

function RepositoryTypeSelector({
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

function SecretSelector({
  value,
  onChange,
  disabled,
  allowNone = false,
}: {
  value?: string | null;
  onChange: (value: string | null) => void;
  disabled?: boolean;
  allowNone?: boolean;
}) {
  const { data, isLoading } = useRead('listSecretDefinitions');
  const secrets = data?.data.secrets ?? [];
  const noneValue = '__none';
  const selectValue = value || (allowNone ? noneValue : undefined);

  return (
    <Select
      value={selectValue}
      disabled={disabled || isLoading || (!allowNone && secrets.length === 0)}
      onValueChange={(next) => onChange(next === noneValue ? null : next)}>
      <SelectTrigger className="w-full max-w-100">
        <SelectValue placeholder={isLoading ? 'Loading secrets...' : 'Select a stored secret'} />
      </SelectTrigger>
      <SelectContent className="bg-background">
        {allowNone && <SelectItem value={noneValue}>No session token</SelectItem>}
        {secrets.map((secret: SecretDefinitionView) => (
          <SelectItem key={secret.id} value={secret.id}>
            <span className="font-medium">{secret.name}</span>
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

function mergeSpec(original: BackupRepositorySpec, update?: BackupRepositorySpec | null): BackupRepositorySpec {
  if (!update) return original;
  if (update.$type && update.$type !== original.$type) return update;
  return { ...original, ...update } as BackupRepositorySpec;
}

function toCreateInput(payload: BackupRepositoryFormInput): BackupRepositoryInput {
  return {
    name: payload.name,
    description: payload.description ?? null,
    passwordSecretId: payload.passwordSecretId,
    spec: normalizeSpec(payload.spec),
  };
}

function toUpdateInput(payload: BackupRepositoryFormInput): UpdateBackupRepositoryInput {
  return {
    description: payload.description ?? null,
    spec: normalizeSpec(payload.spec),
  };
}

function normalizeSpec(spec: BackupRepositorySpec): BackupRepositorySpec {
  if (spec.$type === 'S3Compatible') {
    const s3 = spec as BackupRepositorySpecS3CompatibleBackupRepositorySpec;
    return {
      $type: 'S3Compatible',
      endpoint: s3.endpoint,
      bucket: s3.bucket,
      prefix: s3.prefix || null,
      region: s3.region || null,
      bucketLookup: s3.bucketLookup ?? S3BucketLookup.Auto,
      accessKeySecretId: s3.accessKeySecretId,
      secretKeySecretId: s3.secretKeySecretId,
      sessionTokenSecretId: s3.sessionTokenSecretId || null,
      allowInsecureHttp: s3.allowInsecureHttp ?? false,
    };
  }

  const fileSystem = spec as BackupRepositorySpecFileSystemBackupRepositorySpec;
  return {
    $type: 'FileSystem',
    location: fileSystem.location ?? BackupExecutionLocation.Core,
    platformId: fileSystem.platformId ?? null,
    path: fileSystem.path,
  };
}
