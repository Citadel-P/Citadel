import { useReconcileBackupRepository } from '../backup-repositories/cache';
import {
  BackupRepositoryFormInput,
  createDefaultInput,
  createFileSystemSpec,
  createS3Spec,
  toCreateInput,
  toUpdateInput,
} from '../backup-repositories/input';
import {
  BackupRepositorySpec,
  BackupRepositoryStatus,
  BackupRepositoryType,
  BackupRepositoryView,
  S3BucketLookup,
  BackupExecutionLocation,
  PlatformView,
  ResourceControlState,
} from '@/api/generated/api.types';
import { IntegrationAddCard } from '@/components/custom/common';
import { FieldInput, FieldSwitch, ItemSelector } from '@/components/custom/form-builder';
import { StateIndicator } from '@/components/custom/state-indicator';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Label } from '@/components/ui/label';
import { useMutate } from '@/lib/hooks';
import { cn } from '@/lib/utils';
import {
  CheckCircle2,
  DatabaseBackup,
  LoaderCircle,
  LockKeyhole,
  ChevronDown,
  Plus,
  RefreshCw,
  Scissors,
} from 'lucide-react';
import { type Dispatch, type ReactNode, type SetStateAction, useState } from 'react';
import { toast } from 'sonner';
import { RepositoryCredentials } from '../backup-repositories/credentials';
import { BackupRepositoryDropdownActions } from '../backup-repositories/actions';
import { RepositoryCard } from '../backup-repositories/table';
import { useRepositoryOperations } from '../backup-repositories/operations';
import {
  bucketLookupOptions,
  fileSystemLocationOptions,
  FileSystemPlatformSelector,
  RepositoryTypeSelector,
  SecretSelector,
} from '../backup-repositories/form/form';
import { useBackupRepositorySecrets } from '../backup-repositories/hooks/useBackupRepositorySecrets';
import { useBackupRepositoriesGroup } from '../backup-repositories/hooks/useBackupRepositoriesGroup';

const EMPTY_REPOSITORIES: BackupRepositoryView[] = [];

export function BackupRepositoriesSection() {
  const { repositories: loadedRepositories, isLoading, capabilities } = useBackupRepositoriesGroup();
  const repositories = loadedRepositories ?? EMPTY_REPOSITORIES;
  const canWrite = capabilities?.canWrite ?? false;

  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState<BackupRepositoryView | null>(null);
  const [input, setInput] = useState<BackupRepositoryFormInput>(createDefaultInput);
  const openAdd = () => {
    setEditing(null);
    setInput(createDefaultInput());
    setOpen(true);
  };

  const openEdit = (repository: BackupRepositoryView) => {
    setEditing(repository);
    setInput(getInitialInput(repository));
    setOpen(true);
  };

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex items-center gap-3">
          <div className="inline-flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10 text-primary">
            <DatabaseBackup className="h-4 w-4" />
          </div>
          <div>
            <div className="font-bold">Backup Repositories</div>
            <p className="text-xs text-muted-foreground">Encrypted destinations used by backup policies.</p>
          </div>
        </div>
        <Button variant="outline" disabled={!canWrite} onClick={openAdd}>
          <Plus className="h-3.5 w-3.5" /> Add Repository
        </Button>
      </div>

      {isLoading ? (
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-4">
          {Array.from({ length: 4 }).map((_, index) => (
            <div key={index} className="h-35 animate-pulse rounded-xl border bg-muted/20" />
          ))}
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-4">
          {repositories.map((repository) => (
            <RepositoryCard
              key={repository.id}
              repository={repository}
              actions={BackupRepositoryDropdownActions}
              onEdit={() => openEdit(repository)}
            />
          ))}

          <IntegrationAddCard label="Add Repository" disabled={!canWrite} onClick={openAdd} />
        </div>
      )}

      <RepositoryDialog
        open={open}
        onOpenChange={setOpen}
        editing={editing && (repositories.find((repository) => repository.id === editing.id) ?? editing)}
        setEditing={setEditing}
        input={input}
        setInput={setInput}
      />
    </div>
  );
}

function RepositoryDialog({
  open,
  onOpenChange,
  editing,
  setEditing,
  input,
  setInput,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  editing: BackupRepositoryView | null;
  setEditing: Dispatch<SetStateAction<BackupRepositoryView | null>>;
  input: BackupRepositoryFormInput;
  setInput: Dispatch<SetStateAction<BackupRepositoryFormInput>>;
}) {
  const reconcile = useReconcileBackupRepository();
  const create = useMutate('createBackupRepository', { onSuccess: () => reconcile() });
  const update = useMutate('updateBackupRepository', { onSuccess: (_, { id }) => reconcile(id) });
  const { canRun, run, pending: pendingOperation } = useRepositoryOperations(editing);
  const [updatingCredentials, setUpdatingCredentials] = useState(false);
  const saving = create.isPending || update.isPending;
  const operating =
    updatingCredentials || !!pendingOperation || editing?.controlState === ResourceControlState.Processing;
  const selectedType = (input.spec?.$type ?? BackupRepositoryType.FileSystem) as BackupRepositoryType;
  const formDisabled = !!editing && editing.capabilities?.canWrite !== true;
  const ready = editing?.status === BackupRepositoryStatus.Ready;
  const specDisabled = formDisabled || ready;
  const patch = editing
    ? toUpdateInput(ready ? { ...input, spec: editing.spec } : input, getInitialInput(editing))
    : undefined;
  const hasChanges = !editing || Object.keys(patch ?? {}).length > 0;
  const isS3 = selectedType === BackupRepositoryType.S3Compatible;
  const s3Spec = input.spec as Extract<BackupRepositorySpec, { $type: 'S3Compatible' }>;
  const fileSystemSpec = input.spec as Extract<BackupRepositorySpec, { $type: 'FileSystem' }>;
  const fileSystemLocation = fileSystemSpec.location ?? BackupExecutionLocation.Core;

  const setSpec = (
    patch: Partial<
      Omit<Extract<BackupRepositorySpec, { $type: 'FileSystem' }>, '$type'> &
        Omit<Extract<BackupRepositorySpec, { $type: 'S3Compatible' }>, '$type'>
    >,
  ) =>
    setInput((current) => ({
      ...current,
      spec: { ...current.spec, ...patch } as BackupRepositorySpec,
    }));

  const save = async () => {
    if (saving || operating || formDisabled || !hasChanges) return;
    const error = validateRepositoryInput(input, editing);
    if (error) {
      toast.error(error);
      return;
    }

    try {
      if (editing) {
        await update.mutateAsync({ id: editing.id, data: patch! } as any);
        toast.success('Repository updated successfully');
        onOpenChange(false);
      } else {
        const response = await create.mutateAsync({ data: toCreateInput(input) });
        const repository = (response as any)?.data as BackupRepositoryView | undefined;
        if (repository) {
          setEditing(repository);
          setInput(getInitialInput(repository));
        }
        toast.success('Repository created. Initialize or validate it before assigning it to critical policies.');
      }
    } catch {
      // toast.error(create.validationErrors ?? update.validationErrors ?? 'Failed to save repository.');
    }
  };

  const maintenanceLocked = formDisabled || updatingCredentials || saving || hasChanges;
  const title = editing ? (formDisabled ? 'Backup Repository' : 'Edit Backup Repository') : 'Add Backup Repository';

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!updatingCredentials) onOpenChange(next);
      }}>
      <DialogContent
        className="flex max-h-[90dvh] flex-col gap-0 overflow-hidden p-0 sm:max-w-175"
        onInteractOutside={(e) => e.preventDefault()}>
        <DialogHeader className="border-b px-6 py-5 pr-12 text-left">
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>
            {editing
              ? formDisabled
                ? 'You have read-only access to this repository.'
                : 'Manage repository details, storage and maintenance.'
              : 'Create an encrypted destination that backup policies can use.'}
          </DialogDescription>
        </DialogHeader>

        <div className="min-h-0 flex-1 space-y-6 overflow-y-auto px-6 py-5">
          <section aria-label="Repository details" className="space-y-4">
            <div className="flex items-center justify-between gap-3">
              <h3 className="text-sm font-semibold">Details</h3>
              {editing && <StateIndicator variant="badge" value={editing.status} />}
            </div>
            <div className="space-y-2">
              <Label htmlFor="repository-name">Name</Label>
              <FieldInput
                id="repository-name"
                className="max-w-full"
                value={input.name ?? ''}
                disabled={formDisabled || saving || updatingCredentials}
                placeholder="daily-core-backups"
                onChange={(name) => setInput((current) => ({ ...current, name }))}
              />
            </div>
          </section>
          {ready && editing ? (
            <ReadyRepositoryDetails repository={editing} />
          ) : (
            <section aria-label="Repository storage" className="space-y-5 border-t pt-5">
              <h3 className="text-sm font-semibold">Storage & encryption</h3>
              <div className="space-y-2">
                <Label>Type</Label>
                <RepositoryTypeSelector
                  value={selectedType}
                  disabled={specDisabled || saving}
                  onChange={(type) =>
                    setInput((current) => ({
                      ...current,
                      spec: type === BackupRepositoryType.S3Compatible ? createS3Spec() : createFileSystemSpec(),
                    }))
                  }
                />
              </div>
              <div className="space-y-2">
                <Label>Password Secret</Label>
                <SecretSelector
                  value={input.passwordSecretId}
                  disabled={!!editing || formDisabled}
                  repository={editing}
                  onChange={(passwordSecretId) =>
                    setInput((current) => ({
                      ...current,
                      passwordSecretId: passwordSecretId ?? '',
                    }))
                  }
                />
              </div>

              {editing && (
                <p className="text-xs text-muted-foreground">
                  The encryption password is fixed after creation. Changing the selected secret does not rotate the
                  repository password.
                </p>
              )}

              {!isS3 ? (
                <div className="space-y-5">
                  <div className="space-y-2">
                    <Label>Location</Label>
                    <ItemSelector
                      className="max-w-full"
                      collection={fileSystemLocationOptions}
                      value={fileSystemLocation}
                      disabled={specDisabled}
                      onChange={(location: BackupExecutionLocation) =>
                        setSpec({
                          location,
                          platformId:
                            location === BackupExecutionLocation.Platform ? (fileSystemSpec.platformId ?? null) : null,
                        })
                      }
                    />
                  </div>

                  {fileSystemLocation === BackupExecutionLocation.Platform && (
                    <div className="space-y-2">
                      <Label>Platform</Label>
                      <FileSystemPlatformSelector
                        selected={fileSystemSpec.platformId ?? undefined}
                        disabled={specDisabled}
                        onSelect={(platform: PlatformView | undefined) => setSpec({ platformId: platform?.id ?? null })}
                      />
                      <p className="text-xs text-muted-foreground">
                        Snapshots stay on this Docker host. Swarm backups require an S3-compatible repository.
                      </p>
                    </div>
                  )}

                  <div className="space-y-2">
                    <Label>Repository Path</Label>
                    <FieldInput
                      className="max-w-full"
                      value={fileSystemSpec.path ?? ''}
                      disabled={specDisabled}
                      placeholder={
                        fileSystemLocation === BackupExecutionLocation.Core
                          ? 'daily/core'
                          : '/srv/backups or backup-volume'
                      }
                      onChange={(path) => setSpec({ path })}
                    />
                    <p className="text-xs text-muted-foreground">
                      {fileSystemLocation === BackupExecutionLocation.Core
                        ? 'Relative paths are created under the configured Core backup repository root.'
                        : 'Existing absolute directory or Docker volume name on the selected host. Agent paths are not paths inside Core.'}
                    </p>
                  </div>
                </div>
              ) : (
                <div className="space-y-5">
                  <div className="grid gap-4 md:grid-cols-2">
                    <div className="space-y-2">
                      <Label>Endpoint</Label>
                      <FieldInput
                        className="max-w-full"
                        value={s3Spec.endpoint ?? ''}
                        disabled={specDisabled}
                        placeholder="https://s3.example.com"
                        onChange={(endpoint) => setSpec({ endpoint })}
                      />
                    </div>
                    <div className="space-y-2">
                      <Label>Bucket</Label>
                      <FieldInput
                        className="max-w-full"
                        value={s3Spec.bucket ?? ''}
                        disabled={specDisabled}
                        placeholder="citadel-backups"
                        onChange={(bucket) => setSpec({ bucket })}
                      />
                    </div>
                  </div>

                  <div className="grid gap-4 md:grid-cols-2">
                    <div className="space-y-2">
                      <Label>Prefix</Label>
                      <FieldInput
                        className="max-w-full"
                        value={s3Spec.prefix ?? ''}
                        disabled={specDisabled}
                        placeholder="production/core"
                        onChange={(prefix) => setSpec({ prefix: prefix || null })}
                      />
                    </div>
                    <div className="space-y-2">
                      <Label>Region</Label>
                      <FieldInput
                        className="max-w-full"
                        value={s3Spec.region ?? ''}
                        disabled={specDisabled}
                        placeholder="us-east-1"
                        onChange={(region) => setSpec({ region: region || null })}
                      />
                    </div>
                  </div>

                  <div className="grid gap-4 md:grid-cols-2">
                    <div className="space-y-2">
                      <Label>Bucket Lookup</Label>
                      <ItemSelector
                        className="max-w-full"
                        collection={bucketLookupOptions}
                        value={s3Spec.bucketLookup ?? S3BucketLookup.Auto}
                        disabled={specDisabled}
                        onChange={(bucketLookup: S3BucketLookup) => setSpec({ bucketLookup })}
                      />
                    </div>
                    <div className="flex items-center justify-between gap-4 rounded-lg border p-3">
                      <div className="space-y-0.5">
                        <Label htmlFor="allow-insecure-http">Allow Insecure HTTP</Label>
                        <p className="text-xs text-muted-foreground">
                          Allow plain HTTP endpoints for local object stores.
                        </p>
                      </div>
                      <FieldSwitch
                        id="allow-insecure-http"
                        checked={s3Spec.allowInsecureHttp ?? false}
                        disabled={specDisabled}
                        onChange={(allowInsecureHttp) => setSpec({ allowInsecureHttp })}
                      />
                    </div>
                  </div>

                  <div className="grid gap-4 md:grid-cols-2">
                    <div className="space-y-2">
                      <Label>Access Key Secret</Label>
                      <SecretSelector
                        value={s3Spec.accessKeySecretId}
                        disabled={specDisabled}
                        repository={editing}
                        onChange={(accessKeySecretId) => setSpec({ accessKeySecretId: accessKeySecretId ?? '' })}
                      />
                    </div>
                    <div className="space-y-2">
                      <Label>Secret Key Secret</Label>
                      <SecretSelector
                        value={s3Spec.secretKeySecretId}
                        disabled={specDisabled}
                        repository={editing}
                        onChange={(secretKeySecretId) => setSpec({ secretKeySecretId: secretKeySecretId ?? '' })}
                      />
                    </div>
                  </div>

                  <div className="space-y-2">
                    <Label>Session Token Secret</Label>
                    <SecretSelector
                      value={s3Spec.sessionTokenSecretId}
                      allowNone
                      disabled={specDisabled}
                      repository={editing}
                      onChange={(sessionTokenSecretId) => setSpec({ sessionTokenSecretId })}
                    />
                  </div>
                </div>
              )}
            </section>
          )}

          {ready && editing?.spec.$type === 'S3Compatible' && (
            <RepositoryCredentials
              key={editing.id}
              repository={editing}
              disabled={hasChanges || saving || (operating && !updatingCredentials)}
              onPendingChange={setUpdatingCredentials}
              onUpdated={(repository) => {
                setEditing(repository);
                setInput(getInitialInput(repository));
              }}
            />
          )}

          {editing && (
            <details className="group rounded-lg border">
              <summary className="flex cursor-pointer list-none items-center justify-between gap-3 p-4 text-sm font-medium [&::-webkit-details-marker]:hidden">
                Maintenance
                <ChevronDown className="size-4 text-muted-foreground transition-transform group-open:rotate-180 motion-reduce:transition-none" />
              </summary>
              <div className="space-y-3 border-t p-4">
                {hasChanges && (
                  <p className="text-xs text-muted-foreground">Save your changes before running maintenance.</p>
                )}
                <div className="mb-3 flex flex-col gap-1">
                  <div className="flex items-center gap-2 text-sm font-medium">
                    <StateIndicator value={editing.status} />
                    Repository status
                  </div>
                  <p className="text-xs text-muted-foreground">
                    Initialize a new empty destination once. Validate an existing Restic repository before using it.
                    Check and prune are maintenance actions for ready repositories.
                  </p>
                </div>

                <div className={cn('grid gap-2 sm:grid-cols-2', ready ? 'lg:grid-cols-3' : 'lg:grid-cols-4')}>
                  <RepositoryOperationButton
                    icon={<CheckCircle2 className="size-3.5" />}
                    label="Validate"
                    description="Confirm Citadel can access this repository."
                    disabled={!canRun('validate', maintenanceLocked)}
                    loading={pendingOperation === 'validate'}
                    onClick={() => run('validate', maintenanceLocked)}
                  />
                  {!ready && (
                    <RepositoryOperationButton
                      icon={<DatabaseBackup className="size-3.5" />}
                      label="Initialize"
                      description="Create the Restic repository in an empty destination."
                      disabled={!canRun('initialize', maintenanceLocked)}
                      loading={pendingOperation === 'initialize'}
                      onClick={() => run('initialize', maintenanceLocked)}
                    />
                  )}
                  <RepositoryOperationButton
                    icon={<RefreshCw className="size-3.5" />}
                    label="Check"
                    description="Verify repository integrity."
                    disabled={!canRun('check', maintenanceLocked)}
                    loading={pendingOperation === 'check'}
                    onClick={() => run('check', maintenanceLocked)}
                  />
                  <RepositoryOperationButton
                    icon={<Scissors className="size-3.5" />}
                    label="Prune"
                    description="Reclaim unreferenced data without removing snapshots. Policy retention controls snapshot expiration."
                    disabled={!canRun('prune', maintenanceLocked)}
                    loading={pendingOperation === 'prune'}
                    onClick={() => run('prune', maintenanceLocked)}
                  />
                </div>
              </div>
            </details>
          )}
        </div>

        <DialogFooter className="border-t bg-muted/20 px-6 py-4">
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={saving || updatingCredentials}>
            {formDisabled ? 'Close' : 'Cancel'}
          </Button>
          {!formDisabled && (
            <Button onClick={save} disabled={saving || operating || !hasChanges}>
              {editing ? 'Save changes' : 'Create repository'}{' '}
              {saving && <LoaderCircle className="ml-1 h-3.5 w-3.5 animate-spin" />}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function ReadyRepositoryDetails({ repository }: { repository: BackupRepositoryView }) {
  const { canList, secrets, isLoading } = useBackupRepositorySecrets(repository);
  const secretName = (id?: string | null) => {
    if (!id) return 'None';
    if (!canList) return 'Configured';
    return secrets.find((secret) => secret.id === id)?.name ?? (isLoading ? 'Loading…' : 'Stored secret unavailable');
  };
  const spec = repository.spec;
  const fields: [string, string][] =
    spec.$type === 'S3Compatible'
      ? [
          ['Type', 'S3 compatible'],
          ['Endpoint', spec.endpoint],
          ['Bucket', spec.bucket],
          ['Prefix', spec.prefix || 'Bucket root'],
          ['Region', spec.region || 'Default'],
          ['Bucket lookup', spec.bucketLookup || 'Auto'],
          ['Insecure HTTP', spec.allowInsecureHttp ? 'Allowed' : 'Disabled'],
          ['Access key secret', secretName(spec.accessKeySecretId)],
          ['Secret key secret', secretName(spec.secretKeySecretId)],
          ['Session token secret', secretName(spec.sessionTokenSecretId)],
        ]
      : [
          ['Type', 'Filesystem'],
          ['Location', spec.location || 'Core'],
          ...(spec.platformId ? [['Platform', spec.platformId] as [string, string]] : []),
          ['Repository path', spec.path],
        ];
  fields.push(['Encryption password secret', secretName(repository.passwordSecretId)]);
  return (
    <section aria-label="Repository storage" className="space-y-4 rounded-lg border bg-muted/20 p-4">
      <div className="flex items-start gap-3">
        <LockKeyhole aria-hidden="true" className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
        <div className="space-y-1">
          <h3 className="text-sm font-semibold">Storage & encryption</h3>
          <p className="text-xs leading-relaxed text-muted-foreground">
            This repository is ready. Its destination and selected secrets are locked to keep existing backups
            accessible. The name can still be updated with Write permission.
          </p>
        </div>
      </div>
      <dl className="grid grid-cols-1 gap-x-6 gap-y-4 border-t pt-4 sm:grid-cols-2">
        {fields.map(([label, value]) => (
          <div key={label} className="min-w-0 space-y-1">
            <dt className="text-xs text-muted-foreground">{label}</dt>
            <dd className="text-sm wrap-anywhere">{value}</dd>
          </div>
        ))}
      </dl>
    </section>
  );
}

function RepositoryOperationButton({
  icon,
  label,
  description,
  disabled,
  loading,
  onClick,
}: {
  icon: ReactNode;
  label: string;
  description: string;
  disabled?: boolean;
  loading?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className={cn(
        'flex min-h-22 flex-col items-start gap-1.5 rounded-sm border bg-background p-3 text-left transition-colors hover:border-primary/40 hover:bg-accent/40',
        disabled && 'cursor-not-allowed opacity-60 hover:border-border hover:bg-background',
      )}>
      <span className="inline-flex items-center gap-1.5 text-xs font-medium">
        {loading ? <LoaderCircle className="size-3.5 animate-spin" /> : icon}
        {label}
      </span>
      <span className="text-xs leading-snug text-muted-foreground">{description}</span>
    </button>
  );
}

function getInitialInput(repository: BackupRepositoryView): BackupRepositoryFormInput {
  return {
    ...repository,
    name: repository.name,
    passwordSecretId: repository.passwordSecretId,
    spec: repository.spec,
  };
}

function validateRepositoryInput(input: BackupRepositoryFormInput, editing: BackupRepositoryView | null) {
  if (!input.name.trim()) return 'Repository name is required.';
  if (!editing && !input.passwordSecretId) return 'Password secret is required.';

  if (input.spec.$type === 'S3Compatible') {
    const spec = input.spec as Extract<BackupRepositorySpec, { $type: 'S3Compatible' }>;
    if (!spec.endpoint?.trim()) return 'S3 endpoint is required.';
    if (!spec.bucket?.trim()) return 'S3 bucket is required.';
    if (!spec.accessKeySecretId) return 'Access key secret is required.';
    if (!spec.secretKeySecretId) return 'Secret key secret is required.';
    return null;
  }

  const spec = input.spec as Extract<BackupRepositorySpec, { $type: 'FileSystem' }>;
  if (spec.location === BackupExecutionLocation.Platform && !spec.platformId) return 'Platform is required.';
  if (!spec.path?.trim()) return 'Repository path is required.';
  return null;
}
