import {
  BackupRepositorySpec,
  BackupRepositorySpecFileSystemBackupRepositorySpec,
  BackupRepositorySpecS3CompatibleBackupRepositorySpec,
  BackupRepositoryStatus,
  BackupRepositoryType,
  BackupRepositoryView,
  S3BucketLookup,
  BackupExecutionLocation,
  LookupResourceType,
  PlatformView,
  ResourceControlState,
} from '@/api/generated/api.types';
import { IntegrationAddCard, IntegrationCard, ResourceSelectorField } from '@/components/custom/common';
import { DropdownActionButton, RowActionMenu } from '@/components/custom/dropdown-with-dialog';
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
import { hasCapability } from '@/lib/resource-capabilities';
import { cn } from '@/lib/utils';
import { useQueryClient } from '@tanstack/react-query';
import {
  CheckCircle2,
  Cloud,
  DatabaseBackup,
  Eye,
  FolderLock,
  LoaderCircle,
  Plus,
  RefreshCw,
  Scissors,
} from 'lucide-react';
import { type Dispatch, type ReactNode, type SetStateAction, useState } from 'react';
import { toast } from 'sonner';
import { BackupRepositoryDropdownActions } from '../backup-repositories/actions';
import { destinationText } from '../backup-repositories/table';
import {
  BackupRepositoryFormInput,
  bucketLookupOptions,
  createDefaultInput,
  createFileSystemSpec,
  createS3Spec,
  fileSystemLocationOptions,
  getRepositoryOperationContext,
  RepositoryTypeSelector,
  SecretSelector,
  toCreateInput,
  toUpdateInput,
} from '../backup-repositories/form/form';
import { useBackupRepositoriesGroup } from '../backup-repositories/hooks/useBackupRepositoriesGroup';

const EMPTY_REPOSITORIES: BackupRepositoryView[] = [];
type RepositoryOperation = 'validate' | 'initialize' | 'check' | 'prune';
type PendingRepositoryOperations = Partial<Record<string, RepositoryOperation>>;

export function BackupRepositoriesSection() {
  const { repositories: loadedRepositories, isLoading, capabilities } = useBackupRepositoriesGroup();
  const repositories = loadedRepositories ?? EMPTY_REPOSITORIES;
  const canWrite = capabilities?.canWrite ?? false;

  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState<BackupRepositoryView | null>(null);
  const [input, setInput] = useState<BackupRepositoryFormInput>(createDefaultInput);
  const [pendingOperations, setPendingOperations] = useState<PendingRepositoryOperations>({});

  const setRepositoryOperation = (repositoryId: string, operation: RepositoryOperation | null) => {
    setPendingOperations((current) => {
      if (operation) {
        return { ...current, [repositoryId]: operation };
      }

      const { [repositoryId]: _removed, ...next } = current;
      return next;
    });
  };

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
              pendingOperation={pendingOperations[repository.id] ?? null}
              onPendingOperationChange={setRepositoryOperation}
              onEdit={() => openEdit(repository)}
            />
          ))}

          <IntegrationAddCard label="Add Repository" disabled={!canWrite} onClick={openAdd} />
        </div>
      )}

      <RepositoryDialog
        open={open}
        onOpenChange={setOpen}
        editing={editing}
        setEditing={setEditing}
        input={input}
        setInput={setInput}
      />
    </div>
  );
}

function RepositoryCard({
  repository,
  pendingOperation,
  onPendingOperationChange,
  onEdit,
}: {
  repository: BackupRepositoryView;
  pendingOperation: RepositoryOperation | null;
  onPendingOperationChange: (repositoryId: string, operation: RepositoryOperation | null) => void;
  onEdit: () => void;
}) {
  const queryClient = useQueryClient();
  const validate = useMutate('validateBackupRepository');
  const initialize = useMutate('initializeBackupRepository');
  const check = useMutate('checkBackupRepository');
  const prune = useMutate('pruneBackupRepository');

  const Icon = repository.type === BackupRepositoryType.S3Compatible ? Cloud : FolderLock;
  const label = repository.type === BackupRepositoryType.S3Compatible ? 'S3-compatible' : 'Filesystem';
  const {
    edit: _routeEdit,
    validate: _validateAction,
    initialize: _initializeAction,
    check: _checkAction,
    prune: _pruneAction,
    ...repositoryActions
  } = BackupRepositoryDropdownActions;
  const canExecute = hasCapability(repository, 'canExecute');
  const isResourceProcessing = repository.controlState === ResourceControlState.Processing;
  const operating = pendingOperation !== null || isResourceProcessing;

  const refreshRepository = async () => {
    await queryClient.invalidateQueries({ queryKey: ['listBackupRepositories'] });
    await queryClient.invalidateQueries({ queryKey: ['getBackupRepository', { id: repository.id }] });
  };

  const runOperation = async (
    operation: RepositoryOperation,
    mutation: { mutateAsync: (variables: any) => Promise<unknown>; validationErrors?: string | null },
    successMessage: string,
    failureMessage: string,
  ) => {
    if (operating) return;

    onPendingOperationChange(repository.id, operation);
    try {
      await mutation.mutateAsync({ id: repository.id, data: getRepositoryOperationContext(repository) } as any);
      await refreshRepository();
      toast.success(successMessage);
    } catch {
      toast.error(mutation.validationErrors ?? failureMessage);
    } finally {
      onPendingOperationChange(repository.id, null);
    }
  };

  const operationAction = (
    operation: RepositoryOperation,
    title: string,
    icon: ReactNode,
    mutation: { mutateAsync: (variables: any) => Promise<unknown>; validationErrors?: string | null },
    successMessage: string,
    failureMessage: string,
    readyOnly = false,
  ) => {
    const loading = pendingOperation === operation;
    const disabled = !canExecute || operating || (readyOnly && repository.status !== BackupRepositoryStatus.Ready);
    return (
      <DropdownActionButton
        title={title}
        icon={icon}
        loading={loading}
        disabled={disabled}
        onClick={() => runOperation(operation, mutation, successMessage, failureMessage)}
      />
    );
  };

  const actions = {
    edit: ({ resource }: { resource: BackupRepositoryView }) => (
      <DropdownActionButton
        title="Edit"
        icon={<Eye className="h-4 w-4" />}
        disabled={!(resource.capabilities?.canRead ?? true)}
        onClick={onEdit}
      />
    ),
    validate: () =>
      operationAction(
        'validate',
        'Validate',
        <CheckCircle2 className="h-4 w-4" />,
        validate,
        'Repository validated',
        'Failed to validate repository.',
      ),
    initialize: () =>
      operationAction(
        'initialize',
        'Initialize',
        <DatabaseBackup className="h-4 w-4" />,
        initialize,
        'Repository initialized',
        'Failed to initialize repository.',
      ),
    check: () =>
      operationAction(
        'check',
        'Check',
        <RefreshCw className="h-4 w-4" />,
        check,
        'Repository checked',
        'Failed to check repository.',
        true,
      ),
    prune: () =>
      operationAction(
        'prune',
        'Prune',
        <Scissors className="h-4 w-4" />,
        prune,
        'Repository pruned',
        'Failed to prune repository.',
        true,
      ),
    ...repositoryActions,
  };
  const progressLabel = pendingOperation
    ? {
        validate: 'Validating',
        initialize: 'Initializing',
        check: 'Checking',
        prune: 'Pruning',
      }[pendingOperation]
    : isResourceProcessing
      ? 'Processing'
      : null;

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
        progressLabel ? (
          <div className="flex min-w-0 items-center gap-1.5 text-muted-foreground">
            <LoaderCircle className="h-3.5 w-3.5 animate-spin" />
            <span className="truncate text-xs font-medium">{progressLabel}</span>
          </div>
        ) : (
          <div className="flex min-w-0 items-center gap-1.5">
            <StateIndicator value={repository.status} />
            <span className="truncate text-xs font-medium text-muted-foreground">{label}</span>
          </div>
        )
      }
      footerRight={<RowActionMenu resource={repository} actions={actions} />}
    />
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
  const queryClient = useQueryClient();
  const create = useMutate('createBackupRepository');
  const update = useMutate('updateBackupRepository');
  const validate = useMutate('validateBackupRepository');
  const initialize = useMutate('initializeBackupRepository');
  const check = useMutate('checkBackupRepository');
  const prune = useMutate('pruneBackupRepository');
  const [pendingOperation, setPendingOperation] = useState<RepositoryOperation | null>(null);
  const saving = create.isPending || update.isPending;
  const operating =
    pendingOperation !== null || validate.isPending || initialize.isPending || check.isPending || prune.isPending;

  const selectedType = (input.spec?.$type ?? BackupRepositoryType.FileSystem) as BackupRepositoryType;
  const formDisabled = editing ? !(editing.capabilities?.canWrite ?? true) : false;
  const specDisabled = formDisabled || editing?.status === BackupRepositoryStatus.Ready;
  const isS3 = selectedType === BackupRepositoryType.S3Compatible;
  const s3Spec = input.spec as BackupRepositorySpecS3CompatibleBackupRepositorySpec;
  const fileSystemSpec = input.spec as BackupRepositorySpecFileSystemBackupRepositorySpec;
  const fileSystemLocation = fileSystemSpec.location ?? BackupExecutionLocation.Core;

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

      await queryClient.invalidateQueries({ queryKey: ['listBackupRepositories'] });
      if (editing) await queryClient.invalidateQueries({ queryKey: ['getBackupRepository', { id: editing.id }] });
    } catch {
      toast.error(create.validationErrors ?? update.validationErrors ?? 'Failed to save repository.');
    }
  };

  const runOperation = async (
    operation: RepositoryOperation,
    mutation: { mutateAsync: (variables: any) => Promise<unknown>; validationErrors?: string | null },
    successMessage: string,
    failureMessage: string,
  ) => {
    if (!editing) return;

    setPendingOperation(operation);
    try {
      await mutation.mutateAsync({ id: editing.id, data: getRepositoryOperationContext(editing) } as any);
      await queryClient.invalidateQueries({ queryKey: ['listBackupRepositories'] });
      await queryClient.invalidateQueries({ queryKey: ['getBackupRepository', { id: editing.id }] });
      toast.success(successMessage);
    } catch {
      toast.error(mutation.validationErrors ?? failureMessage);
    } finally {
      setPendingOperation(null);
    }
  };

  const title = editing ? 'Edit Backup Repository' : 'Add Backup Repository';

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="max-h-[85vh] overflow-y-auto sm:max-w-175"
        onInteractOutside={(e) => e.preventDefault()}>
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
                  <ResourceSelectorField
                    targetType={LookupResourceType.Platform}
                    selected={fileSystemSpec.platformId ?? undefined}
                    disabled={specDisabled}
                    onSelect={(platform: PlatformView | undefined) => setSpec({ platformId: platform?.id ?? null })}
                    placeholder="Select Platform"
                  />
                  <p className="text-xs text-muted-foreground">
                    The backup repository is stored on this platform host. Use this for regular or edge agent filesystem
                    backups.
                  </p>
                </div>
              )}

              <div className="space-y-2">
                <Label>Repository Path</Label>
                <FieldInput
                  className="max-w-full"
                  value={fileSystemSpec.path ?? ''}
                  disabled={specDisabled}
                  placeholder={fileSystemLocation === BackupExecutionLocation.Core ? 'daily/core' : '/srv/backups'}
                  onChange={(path) => setSpec({ path })}
                />
                <p className="text-xs text-muted-foreground">
                  {fileSystemLocation === BackupExecutionLocation.Core
                    ? 'Relative paths are created under the configured Core backup repository root.'
                    : 'Use a host path that exists on the selected platform or can be created by the backup runner.'}
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

          {editing && (
            <div className="rounded-lg border bg-muted/10 p-4">
              <div className="mb-3 flex flex-col gap-1">
                <div className="flex items-center gap-2 text-sm font-medium">
                  <StateIndicator value={editing.status} />
                  Repository setup
                </div>
                <p className="text-xs text-muted-foreground">
                  Initialize a new empty destination once. Validate an existing Restic repository before using it. Check
                  and prune are maintenance actions for ready repositories.
                </p>
              </div>

              <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
                <RepositoryOperationButton
                  icon={<CheckCircle2 className="size-3.5" />}
                  label="Validate"
                  description="Confirm Citadel can access this repository."
                  disabled={formDisabled || operating}
                  loading={pendingOperation === 'validate'}
                  onClick={() =>
                    runOperation('validate', validate, 'Repository validated', 'Failed to validate repository.')
                  }
                />
                <RepositoryOperationButton
                  icon={<DatabaseBackup className="size-3.5" />}
                  label="Initialize"
                  description="Create the Restic repository in an empty destination."
                  disabled={formDisabled || operating}
                  loading={pendingOperation === 'initialize'}
                  onClick={() =>
                    runOperation('initialize', initialize, 'Repository initialized', 'Failed to initialize repository.')
                  }
                />
                <RepositoryOperationButton
                  icon={<RefreshCw className="size-3.5" />}
                  label="Check"
                  description="Verify repository integrity."
                  disabled={formDisabled || operating || editing.status !== BackupRepositoryStatus.Ready}
                  loading={pendingOperation === 'check'}
                  onClick={() => runOperation('check', check, 'Repository checked', 'Failed to check repository.')}
                />
                <RepositoryOperationButton
                  icon={<Scissors className="size-3.5" />}
                  label="Prune"
                  description="Remove unreferenced repository data."
                  disabled={formDisabled || operating || editing.status !== BackupRepositoryStatus.Ready}
                  loading={pendingOperation === 'prune'}
                  onClick={() => runOperation('prune', prune, 'Repository pruned', 'Failed to prune repository.')}
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
  if (spec.location === BackupExecutionLocation.Platform && !spec.platformId) return 'Platform is required.';
  if (!spec.path?.trim()) return 'Repository path is required.';
  return null;
}
