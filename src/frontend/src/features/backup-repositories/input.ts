import {
  BackupRepositorySpec,
  BackupExecutionLocation,
  BackupRepositoryInput,
  BackupRepositoryView,
  S3BucketLookup,
  UpdateBackupRepositoryInput,
  RepositoryLocationInput,
} from '@/api/generated/api.types';
export type BackupRepositoryFormInput = BackupRepositoryInput & Partial<BackupRepositoryView>;

export const createFileSystemSpec = (): Extract<BackupRepositorySpec, { $type: 'FileSystem' }> => ({
  $type: 'FileSystem',
  location: BackupExecutionLocation.Core,
  platformId: null,
  path: '',
});

export const createS3Spec = (): Extract<BackupRepositorySpec, { $type: 'S3Compatible' }> => ({
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

export const createDefaultInput = (): BackupRepositoryFormInput => ({
  name: '',
  passwordSecretId: '',
  spec: createFileSystemSpec(),
});

export function mergeSpec(original: BackupRepositorySpec, update?: BackupRepositorySpec | null): BackupRepositorySpec {
  if (!update) return original;
  if (update.$type && update.$type !== original.$type) return update;
  return { ...original, ...update } as BackupRepositorySpec;
}

export function toCreateInput(payload: BackupRepositoryFormInput): BackupRepositoryInput {
  return {
    name: payload.name,
    passwordSecretId: payload.passwordSecretId,
    spec: normalizeSpec(payload.spec),
  };
}

export function toUpdateInput(
  payload: BackupRepositoryFormInput,
  original: BackupRepositoryFormInput,
): UpdateBackupRepositoryInput {
  const patch: UpdateBackupRepositoryInput = {};
  if (payload.name.trim() !== original.name.trim()) patch.name = payload.name.trim();
  const spec = normalizeSpec(payload.spec);
  if (JSON.stringify(spec) !== JSON.stringify(normalizeSpec(original.spec))) patch.spec = spec;
  return patch;
}

export function normalizeSpec(spec: BackupRepositorySpec): BackupRepositorySpec {
  if (spec.$type === 'S3Compatible') {
    const s3 = spec as Extract<BackupRepositorySpec, { $type: 'S3Compatible' }>;
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

  const fileSystem = spec as Extract<BackupRepositorySpec, { $type: 'FileSystem' }>;
  const location = fileSystem.location ?? BackupExecutionLocation.Core;
  return {
    $type: 'FileSystem',
    location,
    platformId: location === BackupExecutionLocation.Platform ? fileSystem.platformId : null,
    path: fileSystem.path,
  };
}

export function getRepositoryOperationContext(repository: BackupRepositoryView): RepositoryLocationInput {
  if (repository.spec.$type === 'FileSystem') {
    const fileSystem = repository.spec as Extract<BackupRepositorySpec, { $type: 'FileSystem' }>;
    const location = fileSystem.location ?? BackupExecutionLocation.Core;
    return {
      location,
      platformId: location === BackupExecutionLocation.Platform ? (fileSystem.platformId ?? null) : null,
    };
  }

  return {
    location: BackupExecutionLocation.Core,
    platformId: null,
  };
}
