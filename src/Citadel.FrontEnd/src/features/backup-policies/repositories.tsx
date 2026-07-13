import {
  BackupRepositorySpec,
  BackupRepositorySpecFileSystemBackupRepositorySpec,
  BackupRepositorySpecS3CompatibleBackupRepositorySpec,
  BackupRepositoryStatus,
  BackupRepositoryType,
  BackupRepositoryView,
  S3BucketLookup,
} from '@/api/generated/api.types';
import { IntegrationAddCard, IntegrationCard } from '@/components/custom/common';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
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
import { useMutate, useRead } from '@/lib/hooks';
import { cn } from '@/lib/utils';
import { useQueryClient } from '@tanstack/react-query';
import { Cloud, DatabaseBackup, FolderLock, LoaderCircle, Plus } from 'lucide-react';
import { type Dispatch, type SetStateAction, useState } from 'react';
import { toast } from 'sonner';
import { BackupRepositoryDropdownActions } from '../backup-repositories/actions';
import { destinationText } from '../backup-repositories/table';
import {
  BackupRepositoryFormInput,
  bucketLookupOptions,
  createDefaultInput,
  createFileSystemSpec,
  createS3Spec,
  RepositoryTypeSelector,
  SecretSelector,
  toCreateInput,
  toUpdateInput,
} from '../backup-repositories/form/form';

const EMPTY_REPOSITORIES: BackupRepositoryView[] = [];

export function BackupRepositoriesSection() {
  const { data, isLoading } = useRead('listBackupRepositories');
  const repositories = data?.data.repositories ?? EMPTY_REPOSITORIES;
  const capabilities = data?.data.capabilities;
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
            <RepositoryCard key={repository.id} repository={repository} onEdit={() => openEdit(repository)} />
          ))}

          <IntegrationAddCard label="Add Repository" disabled={!canWrite} onClick={openAdd} />
        </div>
      )}

      <RepositoryDialog open={open} onOpenChange={setOpen} editing={editing} input={input} setInput={setInput} />
    </div>
  );
}

function RepositoryCard({ repository, onEdit }: { repository: BackupRepositoryView; onEdit: () => void }) {
  const Icon = repository.type === BackupRepositoryType.S3Compatible ? Cloud : FolderLock;
  const label = repository.type === BackupRepositoryType.S3Compatible ? 'S3-compatible' : 'Filesystem';
  const { edit: _edit, ...actions } = BackupRepositoryDropdownActions;

  return (
    <IntegrationCard
      title={repository.name}
      subtitle={destinationText(repository.spec)}
      onEdit={onEdit}
      icon={
        <div
          className={cn(
            'flex h-10 w-10 items-center justify-center rounded-lg border shadow-sm',
            repository.type === BackupRepositoryType.S3Compatible
              ? 'border-sky-500/15 bg-sky-500/10 text-sky-600'
              : 'border-emerald-500/15 bg-emerald-500/10 text-emerald-600',
          )}>
          <Icon className="h-4 w-4" />
        </div>
      }
      footerLeft={
        <div className="flex min-w-0 items-center gap-1.5">
          <StateIndicator value={repository.status} />
          <span className="truncate text-xs font-medium text-muted-foreground">{label}</span>
        </div>
      }
      footerRight={<RowActionMenu resource={repository} actions={actions} />}
    />
  );
}

function RepositoryDialog({
  open,
  onOpenChange,
  editing,
  input,
  setInput,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  editing: BackupRepositoryView | null;
  input: BackupRepositoryFormInput;
  setInput: Dispatch<SetStateAction<BackupRepositoryFormInput>>;
}) {
  const queryClient = useQueryClient();
  const create = useMutate('createBackupRepository');
  const update = useMutate('updateBackupRepository');
  const saving = create.isPending || update.isPending;

  const selectedType = (input.spec?.$type ?? BackupRepositoryType.FileSystem) as BackupRepositoryType;
  const formDisabled = editing ? !(editing.capabilities?.canWrite ?? true) : false;
  const specDisabled = formDisabled || editing?.status === BackupRepositoryStatus.Ready;
  const isS3 = selectedType === BackupRepositoryType.S3Compatible;
  const s3Spec = input.spec as BackupRepositorySpecS3CompatibleBackupRepositorySpec;
  const fileSystemSpec = input.spec as BackupRepositorySpecFileSystemBackupRepositorySpec;

  const setSpec = (
    patch: Partial<
      BackupRepositorySpecFileSystemBackupRepositorySpec & BackupRepositorySpecS3CompatibleBackupRepositorySpec
    >,
  ) =>
    setInput((current) => ({
      ...current,
      spec: { ...current.spec, ...patch } as BackupRepositorySpec,
    }));

  const save = async () => {
    const error = validateRepositoryInput(input, editing);
    if (error) {
      toast.error(error);
      return;
    }

    try {
      if (editing) {
        await update.mutateAsync({ id: editing.id, data: toUpdateInput(input) } as any);
      } else {
        await create.mutateAsync({ data: toCreateInput(input) });
      }

      await queryClient.invalidateQueries({ queryKey: ['listBackupRepositories'] });
      if (editing) await queryClient.invalidateQueries({ queryKey: ['getBackupRepository', { id: editing.id }] });

      toast.success(`Repository ${editing ? 'updated' : 'created'} successfully`);
      onOpenChange(false);
    } catch {
      toast.error(create.validationErrors ?? update.validationErrors ?? 'Failed to save repository.');
    }
  };

  const title = editing ? 'Edit Backup Repository' : 'Add Backup Repository';

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[85vh] overflow-y-auto sm:max-w-175" onInteractOutside={(e) => e.preventDefault()}>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>
            {editing
              ? 'Update repository settings and maintenance configuration.'
              : 'Create an encrypted destination that backup policies can use.'}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-5 py-2">
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label>Name</Label>
              <FieldInput
                className="max-w-full"
                value={input.name ?? ''}
                disabled={!!editing || formDisabled}
                placeholder="daily-core-backups"
                onChange={(name) => setInput((current) => ({ ...current, name }))}
              />
            </div>
            <div className="space-y-2">
              <Label>Type</Label>
              <RepositoryTypeSelector
                value={selectedType}
                disabled={specDisabled}
                onChange={(type) =>
                  setInput((current) => ({
                    ...current,
                    spec: type === BackupRepositoryType.S3Compatible ? createS3Spec() : createFileSystemSpec(),
                  }))
                }
              />
            </div>
          </div>

          <div className="space-y-2">
            <Label>Password Secret</Label>
            <SecretSelector
              value={input.passwordSecretId}
              disabled={!!editing || formDisabled}
              onChange={(passwordSecretId) =>
                setInput((current) => ({
                  ...current,
                  passwordSecretId: passwordSecretId ?? '',
                }))
              }
            />
          </div>

          {specDisabled && (
            <div className="rounded-lg border border-dashed bg-muted/20 p-3 text-xs text-muted-foreground">
              Ready repository destinations are locked. Create a new repository when the destination must change.
            </div>
          )}

          {!isS3 ? (
            <div className="space-y-2">
              <Label>Repository Path</Label>
              <FieldInput
                className="max-w-full"
                value={fileSystemSpec.path ?? ''}
                disabled={specDisabled}
                placeholder="/backups/citadel"
                onChange={(path) => setSpec({ path })}
              />
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
                    <p className="text-xs text-muted-foreground">Allow plain HTTP endpoints for local object stores.</p>
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
                    onChange={(accessKeySecretId) => setSpec({ accessKeySecretId: accessKeySecretId ?? '' })}
                  />
                </div>
                <div className="space-y-2">
                  <Label>Secret Key Secret</Label>
                  <SecretSelector
                    value={s3Spec.secretKeySecretId}
                    disabled={specDisabled}
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
                  onChange={(sessionTokenSecretId) => setSpec({ sessionTokenSecretId })}
                />
              </div>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={saving}>
            Cancel
          </Button>
          <Button onClick={save} disabled={saving || formDisabled}>
            Save {saving && <LoaderCircle className="ml-1 h-3.5 w-3.5 animate-spin" />}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function getInitialInput(repository: BackupRepositoryView): BackupRepositoryFormInput {
  return {
    ...repository,
    name: repository.name,
    description: repository.description ?? null,
    passwordSecretId: repository.passwordSecretId,
    spec: repository.spec,
  };
}

function validateRepositoryInput(input: BackupRepositoryFormInput, editing: BackupRepositoryView | null) {
  if (!editing && !input.name.trim()) return 'Repository name is required.';
  if (!editing && !input.passwordSecretId) return 'Password secret is required.';

  if (input.spec.$type === 'S3Compatible') {
    const spec = input.spec as BackupRepositorySpecS3CompatibleBackupRepositorySpec;
    if (!spec.endpoint?.trim()) return 'S3 endpoint is required.';
    if (!spec.bucket?.trim()) return 'S3 bucket is required.';
    if (!spec.accessKeySecretId) return 'Access key secret is required.';
    if (!spec.secretKeySecretId) return 'Secret key secret is required.';
    return null;
  }

  const spec = input.spec as BackupRepositorySpecFileSystemBackupRepositorySpec;
  if (!spec.path?.trim()) return 'Repository path is required.';
  return null;
}
