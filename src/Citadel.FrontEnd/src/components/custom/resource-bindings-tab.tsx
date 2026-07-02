import {
  ResourceBindingInput,
  ResourceBindingKind,
  ResourceBindingView,
  ResourceBindingScope,
  CreateExternalSecretInput,
  CreateInternalSecretInput,
  SecretProviderType,
  TestExternalSecretInput,
  SecretDefinitionView,
  SecretProviderView,
  UpdateExternalSecretInput,
  UpdateResourceBindingInput,
} from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { useMutate, useRead } from '@/lib/hooks';
import { cn } from '@/lib/utils';
import {
  ChevronDown,
  ExternalLink,
  KeyRound,
  LoaderCircle,
  MoreHorizontal,
  Pencil,
  Plus,
  Trash2,
  Variable,
} from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import { Link } from 'react-router';

export const ENV_DELIVERY_MODE = 'EnvironmentVariable';
export const MOUNTED_FILE_DELIVERY_MODE = 'MountedFile';
const INTERNAL_SECRET_INPUT: CreateInternalSecretInput = { name: '', value: '' };
const EXTERNAL_SECRET_INPUT: CreateExternalSecretInput = {
  name: '',
  providerId: '',
  externalPath: '',
  externalKey: '',
  externalVersion: null,
};
type SecretSource = 'internal' | 'vault';

export type EditableEntry = ResourceBindingInput & {
  clientId: string;
};

const EMPTY_RESOURCE_BINDINGS: ResourceBindingView[] = [];
const EMPTY_SECRET_DEFINITIONS: SecretDefinitionView[] = [];

export const ResourceBindingSummary = ({
  scope,
  resourceId,
  title = 'Bindings',
  description = 'Resource-specific values are managed in the Bindings tab. Global bindings are resolved during deploy.',
}: {
  scope: ResourceBindingScope.Stack | ResourceBindingScope.Deployment;
  resourceId: string;
  title?: string;
  description?: string;
}) => {
  const args = useMemo(() => ({ scope, resourceId }), [resourceId, scope]);
  const { data, isLoading } = useRead('getResourceBindings', args);
  const effectiveEntries = data?.data.effectiveEntries ?? [];
  const variableCount = effectiveEntries.filter((entry) => entry.kind === ResourceBindingKind.Variable).length;
  const secretCount = effectiveEntries.filter((entry) => entry.kind === ResourceBindingKind.Secret).length;

  const openEnvironmentTab = () => {
    window.location.hash = 'bindings';
  };

  return (
    <div className="rounded-sm border border-dashed bg-muted/20 px-4 py-3">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex flex-col gap-1">
          <div className="text-sm font-medium">{title}</div>
          <div className="text-xs text-muted-foreground">{description}</div>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <span className="inline-flex items-center gap-1 rounded-sm border px-2 py-1 text-xs text-muted-foreground">
            <Variable className="size-3" />
            {isLoading ? '-' : variableCount} effective variables
          </span>
          <span className="inline-flex items-center gap-1 rounded-sm border border-dashed px-2 py-1 text-xs text-muted-foreground">
            <KeyRound className="size-3" />
            {isLoading ? '-' : secretCount} effective secrets
          </span>
          <Button type="button" variant="outline" size="sm" onClick={openEnvironmentTab}>
            Open Bindings
          </Button>
        </div>
      </div>
    </div>
  );
};

export const ResourceBindingsTab = ({
  scope,
  resourceId,
  disabled,
}: {
  scope: ResourceBindingScope.Stack | ResourceBindingScope.Deployment;
  resourceId: string;
  disabled?: boolean;
}) => {
  const args = useMemo(() => ({ scope, resourceId }), [resourceId, scope]);
  const secretQueryArgs = useMemo(() => ({ query: { scope, resourceId } }), [resourceId, scope]);
  const { data, isLoading } = useRead('getResourceBindings', args);
  const { data: secretsData } = useRead('listSecretDefinitions', secretQueryArgs);
  const serverEntries = data?.data.entries ?? EMPTY_RESOURCE_BINDINGS;
  const secrets = useMemo(() => secretsData?.data.secrets ?? EMPTY_SECRET_DEFINITIONS, [secretsData?.data.secrets]);
  const canCreateSecret = Boolean(secretsData?.data.capabilities.canWrite);
  const initialEntries = useMemo(() => serverEntries.map(toEditableEntry), [serverEntries]);
  const originalInputs = useMemo(() => serverEntries.map(toInputFromView), [serverEntries]);
  const resetKey = useMemo(() => JSON.stringify(originalInputs), [originalInputs]);

  return (
    <ResourceBindingsTabEditor
      key={resetKey}
      scope={scope}
      resourceId={resourceId}
      queryArgs={args}
      disabled={disabled}
      isLoading={isLoading}
      initialEntries={initialEntries}
      originalInputs={originalInputs}
      secrets={secrets}
      canCreateSecret={canCreateSecret}
      allowMountedFile={scope === ResourceBindingScope.Stack}
    />
  );
};

const ResourceBindingsTabEditor = ({
  scope,
  resourceId,
  queryArgs,
  disabled,
  isLoading,
  initialEntries,
  originalInputs,
  secrets,
  canCreateSecret,
  allowMountedFile,
}: {
  scope: ResourceBindingScope.Stack | ResourceBindingScope.Deployment;
  resourceId: string;
  queryArgs: { scope: ResourceBindingScope.Stack | ResourceBindingScope.Deployment; resourceId: string };
  disabled?: boolean;
  isLoading: boolean;
  initialEntries: EditableEntry[];
  originalInputs: ResourceBindingInput[];
  secrets: SecretDefinitionView[];
  canCreateSecret: boolean;
  allowMountedFile?: boolean;
}) => {
  const queryClient = useQueryClient();
  const create = useMutate('createResourceBinding');
  const update = useMutate('updateResourceBinding');
  const remove = useMutate('deleteResourceBinding');
  const updateExternalSecret = useMutate('updateExternalSecret');
  const testExternalSecret = useMutate('testExternalSecret');
  const entries = initialEntries;
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingEntry, setEditingEntry] = useState<EditableEntry | null>(null);
  const [entryInput, setEntryInput] = useState<ResourceBindingInput>(newVariableInput());
  const [deleteEntry, setDeleteEntry] = useState<EditableEntry | null>(null);
  const [editingStoredSecret, setEditingStoredSecret] = useState<SecretDefinitionView | null>(null);
  const [storedSecretInput, setStoredSecretInput] = useState<CreateExternalSecretInput>(EXTERNAL_SECRET_INPUT);
  const { data: providersData } = useRead('listSecretProviders', undefined, { enabled: editingStoredSecret != null });
  const providers = useMemo(() => providersData?.data.providers ?? [], [providersData?.data.providers]);

  const updateEntry = async (entryId: string, entry: ResourceBindingInput, message: string) => {
    try {
      await update.mutateAsync({
        scope,
        resourceId,
        data: toUpdateInput(entryId, entry),
      } as any);
      await queryClient.invalidateQueries({ queryKey: ['getResourceBindings', queryArgs] });
      await queryClient.invalidateQueries({ queryKey: ['listSecretDefinitions'] });
      toast.success(message);
    } catch {
      toast.error(update.validationErrors ?? 'Failed to update resource binding');
      throw new Error('Failed to update resource binding');
    }
  };

  const deleteEntryById = async (entryId: string, message: string) => {
    try {
      await remove.mutateAsync({
        scope,
        resourceId,
        id: entryId,
      } as any);
      await queryClient.invalidateQueries({ queryKey: ['getResourceBindings', queryArgs] });
      await queryClient.invalidateQueries({ queryKey: ['listSecretDefinitions'] });
      toast.success(message);
    } catch {
      toast.error(remove.validationErrors ?? 'Failed to delete resource binding');
      throw new Error('Failed to delete resource binding');
    }
  };

  const createEntry = useCallback(async (entry: ResourceBindingInput, message: string) => {
    try {
      await create.mutateAsync({
        scope,
        resourceId,
        data: entry,
      } as any);
      await queryClient.invalidateQueries({ queryKey: ['getResourceBindings', queryArgs] });
      await queryClient.invalidateQueries({ queryKey: ['listSecretDefinitions'] });
      toast.success(message);
      return true;
    } catch {
      toast.error(create.validationErrors ?? 'Failed to create variable or secret');
      return false;
    }
  }, [create, queryClient, queryArgs, resourceId, scope]);

  const openAddVariable = () => {
    setEditingEntry(null);
    setEntryInput(newVariableInput());
    setDialogOpen(true);
  };

  const openAddSecret = () => {
    setEditingEntry(null);
    setEntryInput(newSecretInput(secrets[0]));
    setDialogOpen(true);
  };

  const openEdit = (entry: EditableEntry) => {
    setEditingEntry(entry);
    setEntryInput(toInput(entry));
    setDialogOpen(true);
  };

  const openEditStoredSecret = (secret: SecretDefinitionView) => {
    if (secret.providerType !== SecretProviderType.VaultCompatibleKvV2) {
      toast.error('Only Vault-compatible stored secrets can be edited here.');
      return;
    }

    setEditingStoredSecret(secret);
    setStoredSecretInput(toExternalSecretInput(secret));
  };

  const addSecretBinding = useCallback(
    async (secret?: SecretDefinitionView) => {
      if (!secret) return false;
      const next = [...originalInputs, newSecretInput(secret)];
      if (hasDuplicateEntryName(next)) {
        toast.error('A variable or secret key with this name already exists on this resource.');
        return false;
      }

      return await createEntry(newSecretInput(secret), 'Secret added');
    },
    [createEntry, originalInputs],
  );
  const secretCreation = useSecretCreation(addSecretBinding, secrets);

  const saveDialogEntry = async () => {
    const normalized = normalizeEntryInput(entryInput);
    const next =
      editingEntry == null
        ? [...originalInputs, normalized]
        : originalInputs.map((entry, index) =>
            entries[index].clientId === editingEntry.clientId ? normalized : entry,
          );

    if (hasDuplicateEntryName(next)) {
      toast.error('A variable or secret key with this name already exists on this resource.');
      return;
    }

    if (editingEntry == null) {
      if (!await createEntry(normalized, 'Resource binding added')) return;
    } else {
      await updateEntry(editingEntry.clientId, normalized, 'Resource binding updated');
    }
    setDialogOpen(false);
    setEditingEntry(null);
  };

  const confirmDelete = async () => {
    if (!deleteEntry) return;
    await deleteEntryById(deleteEntry.clientId, 'Resource binding deleted');
    setDeleteEntry(null);
  };

  const saveStoredSecret = async () => {
    if (!editingStoredSecret) return;

    const payload: UpdateExternalSecretInput = normalizeExternalSecretInput(storedSecretInput);
    try {
      const result = await updateExternalSecret.mutateAsync({
        id: editingStoredSecret.id,
        data: payload,
      } as any);
      setEditingStoredSecret(null);
      await queryClient.invalidateQueries({ queryKey: ['listSecretDefinitions'] });
      await queryClient.invalidateQueries({ queryKey: ['getResourceBindings', queryArgs] });
      toast.success('Stored secret updated');

      if (entryInput.secretId === editingStoredSecret.id) {
        setEntryInput((prev) => ({ ...prev, name: prev.name || result.data.name }));
      }
    } catch {
      toast.error(updateExternalSecret.validationErrors ?? 'Failed to update stored secret');
    }
  };

  const testStoredSecret = async () => {
    try {
      const result = await testExternalSecret.mutateAsync({
        data: toExternalSecretTestInput(storedSecretInput),
      } as any);

      if (result.data.success) {
        toast.success(result.data.message);
      } else {
        toast.error(result.data.message);
      }
    } catch {
      toast.error(testExternalSecret.validationErrors ?? 'Failed to test external secret');
    }
  };

  return (
    <div className="flex w-full flex-col gap-4">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
        <div className="text-sm text-muted-foreground">
          Define resource-specific variables and secrets. Values here override global Bindings entries with the same
          name.
        </div>
        <div className="flex flex-wrap gap-2">
          <ResourceBindingAddDropdown
            disabled={disabled}
            hasSecrets={secrets.length > 0}
            canCreateSecret={canCreateSecret}
            onAddVariable={openAddVariable}
            onBindSecret={openAddSecret}
            onCreateSecret={() => secretCreation.setOpen(true)}
          />
        </div>
      </div>

      <ResourceBindingsTable
        entries={entries}
        secrets={secrets}
        isLoading={isLoading}
        emptyText="No resource variables or secrets are defined."
        disabled={disabled}
        onEdit={openEdit}
        onDelete={setDeleteEntry}
      />

      <GlobalBindingsLink />

      <ResourceBindingDialog
        open={dialogOpen}
        input={entryInput}
        secrets={secrets}
        isPending={update.isPending || create.isPending}
        editing={editingEntry != null}
        allowMountedFile={allowMountedFile}
        onOpenChange={setDialogOpen}
        onInputChange={setEntryInput}
        onSave={saveDialogEntry}
        onEditStoredSecret={openEditStoredSecret}
      />
      <EditExternalSecretDialog
        open={editingStoredSecret != null}
        input={storedSecretInput}
        providers={providers}
        isPending={updateExternalSecret.isPending}
        isTesting={testExternalSecret.isPending}
        onOpenChange={(open) => !open && setEditingStoredSecret(null)}
        onInputChange={setStoredSecretInput}
        onSave={saveStoredSecret}
        onTest={testStoredSecret}
      />
      <CreateSecretDialog {...secretCreation.dialogProps} />
      <Dialog open={deleteEntry != null} onOpenChange={(open) => !open && setDeleteEntry(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete resource binding</DialogTitle>
            <DialogDescription>
              Delete `{deleteEntry?.name}` from this resource. Global entries with the same name will become effective
              again.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteEntry(null)} disabled={remove.isPending}>
              Cancel
            </Button>
            <Button variant="destructive" onClick={confirmDelete} disabled={remove.isPending}>
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
};

const ResourceBindingsTable = ({
  entries,
  secrets,
  isLoading,
  emptyText,
  disabled,
  onEdit,
  onDelete,
}: {
  entries: EditableEntry[];
  secrets: SecretDefinitionView[];
  isLoading: boolean;
  emptyText: string;
  disabled?: boolean;
  onEdit: (entry: EditableEntry) => void;
  onDelete: (entry: EditableEntry) => void;
}) => {
  const secretNames = useMemo(() => new Map(secrets.map((secret) => [secret.id, secret.name])), [secrets]);

  if (isLoading) {
    return <div className="rounded-sm border border-dashed px-4 py-6 text-sm text-muted-foreground">Loading...</div>;
  }

  if (!entries.length) {
    return (
      <div className="rounded-sm border border-dashed px-4 py-6 text-center text-sm text-muted-foreground">
        {emptyText}
      </div>
    );
  }

  return (
    <div className="overflow-hidden rounded-sm border">
      <table className="w-full text-sm">
        <thead className="border-b bg-muted/30 text-left text-xs text-muted-foreground">
          <tr>
            <th className="px-3 py-2 font-normal">Name</th>
            <th className="px-3 py-2 font-normal">Type</th>
            <th className="px-3 py-2 font-normal">Value</th>
            <th className="px-3 py-2 font-normal">Delivery</th>
            <th className="w-10 px-2 py-2" />
          </tr>
        </thead>
        <tbody>
          {entries.map((entry) => (
            <tr key={entry.clientId} className="border-b last:border-0">
              <td className="px-3 py-2 align-middle">
                <span className="font-mono text-xs">{entry.name}</span>
              </td>
              <td className="px-3 py-2 align-middle">
                <div
                  className={cn(
                    'inline-flex h-9 items-center gap-2 rounded-sm border px-3 text-xs text-muted-foreground',
                    entry.kind === ResourceBindingKind.Secret ? 'border-dashed' : '',
                  )}>
                  {entry.kind === ResourceBindingKind.Secret ? (
                    <>
                      <KeyRound className="size-3.5" />
                      Secret key
                    </>
                  ) : (
                    <>
                      <Variable className="size-3.5" />
                      Variable
                    </>
                  )}
                </div>
              </td>
              <td className="px-3 py-2 align-middle">
                {entry.kind === ResourceBindingKind.Variable ? (
                  <span className="font-mono text-xs">{entry.value}</span>
                ) : (
                  <div className="flex flex-col gap-0.5">
                    <span className="font-mono text-xs">
                      {entry.secretId ? (secretNames.get(entry.secretId) ?? 'Unknown secret') : 'Unbound secret'}
                    </span>
                    <span className="font-mono text-xs text-muted-foreground">********</span>
                  </div>
                )}
              </td>
              <td className="px-3 py-2 align-middle">
                {entry.kind === ResourceBindingKind.Secret ? (
                  <div className="flex flex-col gap-0.5 text-xs text-muted-foreground">
                    <span>
                      {entry.secretDeliveryMode === MOUNTED_FILE_DELIVERY_MODE
                        ? 'Mounted file'
                        : 'Environment variable'}
                    </span>
                    {entry.secretDeliveryMode === MOUNTED_FILE_DELIVERY_MODE && entry.targetPath && (
                      <span className="font-mono">{entry.targetPath}</span>
                    )}
                  </div>
                ) : (
                  <span className="text-xs text-muted-foreground">-</span>
                )}
              </td>
              <td className="px-2 py-2 align-middle">
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button type="button" variant="ghost" size="icon" disabled={disabled}>
                      <MoreHorizontal className="size-4" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end" className="w-38 bg-background py-2">
                    <DropdownMenuItem onClick={() => onEdit(entry)}>
                      <Pencil className="size-3.5" />
                      Edit
                    </DropdownMenuItem>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem className="text-danger hover:text-danger!" onClick={() => onDelete(entry)}>
                      <Trash2 className="size-3.5" />
                      Delete
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};

const GlobalBindingsLink = () => (
  <div className="flex items-center justify-between rounded-sm border border-dashed px-4 py-3 text-sm">
    <div>
      <div className="font-medium">Global Bindings</div>
      <div className="text-xs text-muted-foreground">
        Global entries are inherited automatically and resolved during deploy. They are not listed in this resource
        table.
      </div>
    </div>
    <Button asChild variant="outline" size="sm">
      <Link to="/bindings">
        <ExternalLink className="size-3.5" />
        Open Bindings
      </Link>
    </Button>
  </div>
);

const ResourceBindingDialog = ({
  open,
  input,
  secrets,
  isPending,
  editing,
  allowMountedFile,
  onOpenChange,
  onInputChange,
  onSave,
  onEditStoredSecret,
}: {
  open: boolean;
  input: ResourceBindingInput;
  secrets: SecretDefinitionView[];
  isPending: boolean;
  editing: boolean;
  allowMountedFile?: boolean;
  onOpenChange: (open: boolean) => void;
  onInputChange: (
    input: ResourceBindingInput | ((prev: ResourceBindingInput) => ResourceBindingInput),
  ) => void;
  onSave: () => void;
  onEditStoredSecret?: (secret: SecretDefinitionView) => void;
}) => {
  const isSecret = input.kind === ResourceBindingKind.Secret;
  const isMountedFile = input.secretDeliveryMode === MOUNTED_FILE_DELIVERY_MODE;
  const selectedSecret = isSecret ? secrets.find((secret) => secret.id === input.secretId) : undefined;
  const mountedFileTargetPathError = isMountedFile ? getMountedFileTargetPathError(input.targetPath) : null;
  const canSave =
    input.name.trim().length > 0 &&
    (!isSecret || (Boolean(input.secretId) && (!isMountedFile || mountedFileTargetPathError == null)));

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-137.5" onInteractOutside={(event) => event.preventDefault()}>
        <DialogHeader>
          <DialogTitle>{editing ? 'Edit entry' : isSecret ? 'Add secret key' : 'Add variable'}</DialogTitle>
          <DialogDescription>
            {isSecret
              ? 'Add a resource-specific runtime key that points to a stored secret.'
              : 'Add a resource-specific variable. It overrides a global variable with the same name.'}
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-4 py-4">
          <div>
            <Label>Name</Label>
            <Input
              value={input.name}
              placeholder="DATABASE_URL"
              onChange={(event) => onInputChange((prev) => ({ ...prev, name: event.target.value }))}
            />
          </div>
          {isSecret ? (
            <>
              <div>
                <Label>Secret</Label>
                <div className="flex gap-2">
                  <Select
                    value={input.secretId ?? ''}
                    disabled={secrets.length === 0}
                    onValueChange={(secretId) => onInputChange((prev) => ({ ...prev, secretId }))}>
                    <SelectTrigger className="flex-1">
                      <SelectValue placeholder={secrets.length ? 'Select secret' : 'No secrets'} />
                    </SelectTrigger>
                    <SelectContent>
                      {secrets.map((secret) => (
                        <SelectItem key={secret.id} value={secret.id}>
                          {secret.name}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  {selectedSecret?.providerType === SecretProviderType.VaultCompatibleKvV2 && onEditStoredSecret && (
                    <Button type="button" variant="outline" onClick={() => onEditStoredSecret(selectedSecret)}>
                      Edit Stored Secret
                    </Button>
                  )}
                </div>
              </div>
              {allowMountedFile && (
                <div>
                  <Label>Delivery</Label>
                  <Select
                    value={input.secretDeliveryMode ?? ENV_DELIVERY_MODE}
                    onValueChange={(secretDeliveryMode) =>
                      onInputChange((prev) => ({
                        ...prev,
                        secretDeliveryMode,
                        targetPath: secretDeliveryMode === MOUNTED_FILE_DELIVERY_MODE ? (prev.targetPath ?? '') : null,
                      }))
                    }>
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value={ENV_DELIVERY_MODE}>Environment variable</SelectItem>
                      <SelectItem value={MOUNTED_FILE_DELIVERY_MODE}>Mounted file</SelectItem>
                    </SelectContent>
                  </Select>
                  <div className="mt-1 text-xs text-muted-foreground">
                    Environment variables are used for compose interpolation. Mounted files are read-only container
                    files for images that support *_FILE settings.
                  </div>
                </div>
              )}
              {allowMountedFile && isMountedFile && (
                <div>
                  <Label>Target path</Label>
                  <Input
                    value={input.targetPath ?? ''}
                    placeholder="/run/secrets/postgres_password"
                    onChange={(event) => onInputChange((prev) => ({ ...prev, targetPath: event.target.value }))}
                  />
                  <div className="mt-1 text-xs text-muted-foreground">
                    Absolute container file path, for example /run/secrets/postgres_password. Reference this path from a
                    compose *_FILE environment variable when the image supports it.
                  </div>
                  {mountedFileTargetPathError && (
                    <div className="mt-1 text-xs text-destructive">{mountedFileTargetPathError}</div>
                  )}
                </div>
              )}
            </>
          ) : (
            <div>
              <Label>Value</Label>
              <Input
                value={input.value ?? ''}
                placeholder="Value"
                onChange={(event) => onInputChange((prev) => ({ ...prev, value: event.target.value }))}
              />
            </div>
          )}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={isPending}>
            Cancel
          </Button>
          <Button onClick={onSave} disabled={!canSave || isPending}>
            Save
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export const EditExternalSecretDialog = ({
  open,
  input,
  providers,
  isPending,
  isTesting,
  onOpenChange,
  onInputChange,
  onSave,
  onTest,
}: {
  open: boolean;
  input: CreateExternalSecretInput;
  providers: SecretProviderView[];
  isPending: boolean;
  isTesting: boolean;
  onOpenChange: (open: boolean) => void;
  onInputChange: (
    input: CreateExternalSecretInput | ((prev: CreateExternalSecretInput) => CreateExternalSecretInput),
  ) => void;
  onSave: () => void;
  onTest: () => void;
}) => {
  const canSave = Boolean(
    input.name.trim() && input.providerId && input.externalPath.trim() && input.externalKey.trim(),
  );

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-137.5" onInteractOutside={(e) => e.preventDefault()}>
        <DialogHeader>
          <DialogTitle>Edit Stored Secret</DialogTitle>
          <DialogDescription>
            Update the Vault reference used by resource secret keys. Existing bindings keep pointing to this stored
            secret.
          </DialogDescription>
        </DialogHeader>
        <ExternalSecretFields input={input} providers={providers} onInputChange={onInputChange} />
        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={isPending || isTesting}>
            Cancel
          </Button>
          <Button type="button" variant="outline" onClick={onTest} disabled={!canSave || isPending || isTesting}>
            Test {isTesting && <LoaderCircle className="ml-1 h-3.5 w-3.5 animate-spin" />}
          </Button>
          <Button type="button" onClick={onSave} disabled={!canSave || isPending || isTesting}>
            Save {isPending && <LoaderCircle className="ml-1 h-3.5 w-3.5 animate-spin" />}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

const ExternalSecretFields = ({
  input,
  providers,
  onInputChange,
}: {
  input: CreateExternalSecretInput;
  providers: SecretProviderView[];
  onInputChange: (
    input: CreateExternalSecretInput | ((prev: CreateExternalSecretInput) => CreateExternalSecretInput),
  ) => void;
}) => (
  <div className="grid gap-4">
    <LabeledInput
      label="Name"
      value={input.name}
      onChange={(name) => onInputChange((prev) => ({ ...prev, name }))}
      placeholder="API_KEY"
      description="Default runtime key used when this secret is added to a resource."
    />
    <div className="grid gap-1.5 text-sm">
      <span className="text-xs text-muted-foreground">Provider</span>
      <Select
        value={input.providerId}
        disabled={providers.length === 0}
        onValueChange={(providerId) => onInputChange((prev) => ({ ...prev, providerId }))}>
        <SelectTrigger>
          <SelectValue placeholder={providers.length ? 'Select provider' : 'No providers'} />
        </SelectTrigger>
        <SelectContent>
          {providers.map((provider) => (
            <SelectItem key={provider.id} value={provider.id}>
              {provider.name}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <span className="text-xs text-muted-foreground">
        Create providers in the Secret Providers section. The secret value is fetched only during apply.
      </span>
    </div>
    <LabeledInput
      label="Path"
      value={input.externalPath}
      onChange={(externalPath) => onInputChange((prev) => ({ ...prev, externalPath }))}
      placeholder="apps/api/prod"
      description="Path inside the selected mount. Do not include the mount name, /v1, or /data."
    />
    <LabeledInput
      label="Key"
      value={input.externalKey}
      onChange={(externalKey) => onInputChange((prev) => ({ ...prev, externalKey }))}
      placeholder="api_key"
      description="Field name inside the Vault secret data object."
    />
    <LabeledInput
      label="Version"
      type="number"
      value={input.externalVersion?.toString() ?? ''}
      onChange={(value) => onInputChange((prev) => ({ ...prev, externalVersion: value ? Number(value) : null }))}
      placeholder="Latest"
      description="Leave empty to resolve the latest version."
    />
  </div>
);

const getMountedFileTargetPathError = (targetPath?: string | null): string | null => {
  const value = targetPath?.trim();
  if (!value) return 'Target path is required.';
  if (!value.startsWith('/') || value.includes('\\')) return 'Use an absolute Linux container path.';
  if (value === '/' || value.endsWith('/')) return 'Target path must point to a file.';
  if (value.split('/').some((segment) => segment === '.' || segment === '..')) {
    return 'Target path cannot contain relative path segments.';
  }
  if (
    value === '/etc/passwd' ||
    value === '/etc/shadow' ||
    value.startsWith('/proc/') ||
    value.startsWith('/sys/') ||
    value.startsWith('/dev/')
  ) {
    return 'Target path uses a protected container path.';
  }

  return null;
};

const LabeledInput = ({
  label,
  value,
  onChange,
  placeholder,
  type = 'text',
  description,
  disabled,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  type?: string;
  description?: string;
  disabled?: boolean;
}) => (
  <label className="grid gap-1.5 text-sm">
    <span className="text-xs text-muted-foreground">{label}</span>
    <Input
      type={type}
      value={value}
      disabled={disabled}
      placeholder={placeholder}
      onChange={(event) => onChange(event.target.value)}
    />
    {description && <span className="text-xs text-muted-foreground">{description}</span>}
  </label>
);

export const ResourceBindingAddDropdown = ({
  disabled,
  hasSecrets,
  canCreateSecret,
  onAddVariable,
  onBindSecret,
  onCreateSecret,
}: {
  disabled?: boolean;
  hasSecrets: boolean;
  canCreateSecret: boolean;
  onAddVariable: () => void;
  onBindSecret: () => void;
  onCreateSecret: () => void;
}) => (
  <DropdownMenu>
    <DropdownMenuTrigger asChild>
      <Button type="button" variant="outline" size="sm" disabled={disabled}>
        <Plus className="size-3.5" />
        Add
        <ChevronDown className="size-3.5" />
      </Button>
    </DropdownMenuTrigger>
    <DropdownMenuContent align="end" className="min-w-40">
      <DropdownMenuItem onClick={onAddVariable}>
        <Variable className="size-3.5" />
        Add variable
      </DropdownMenuItem>
      <DropdownMenuItem disabled={!hasSecrets} onClick={onBindSecret}>
        <KeyRound className="size-3.5" />
        Add secret key
      </DropdownMenuItem>
      <DropdownMenuItem disabled={!canCreateSecret} onClick={onCreateSecret}>
        <Plus className="size-3.5" />
        Create stored secret
      </DropdownMenuItem>
    </DropdownMenuContent>
  </DropdownMenu>
);

export const useSecretCreation = (
  addSecretBinding: (secret?: SecretDefinitionView) => boolean | Promise<boolean>,
  existingSecrets: SecretDefinitionView[] = EMPTY_SECRET_DEFINITIONS,
) => {
  const queryClient = useQueryClient();
  const createInternalSecret = useMutate('createInternalSecret');
  const createExternalSecret = useMutate('createExternalSecret');
  const testExternalSecret = useMutate('testExternalSecret');
  const [open, setOpen] = useState(false);
  const [source, setSource] = useState<SecretSource>('internal');
  const [internalInput, setInternalInput] = useState<CreateInternalSecretInput>(INTERNAL_SECRET_INPUT);
  const [externalInput, setExternalInput] = useState<CreateExternalSecretInput>(EXTERNAL_SECRET_INPUT);
  const { data: providersData } = useRead('listSecretProviders', undefined, { enabled: open && source === 'vault' });
  const providers = useMemo(() => providersData?.data.providers ?? [], [providersData?.data.providers]);

  useEffect(() => {
    if (externalInput.providerId || providers.length === 0) return;
    setExternalInput((prev) => ({ ...prev, providerId: providers[0].id }));
  }, [externalInput.providerId, providers]);

  const resetSecretInputs = () => {
    setInternalInput(INTERNAL_SECRET_INPUT);
    setExternalInput({ ...EXTERNAL_SECRET_INPUT, providerId: providers[0]?.id ?? '' });
  };

  const createSecret = async () => {
    const requestedName = (source === 'internal' ? internalInput.name : externalInput.name).trim();
    const existingSecret = existingSecrets.find((secret) => secret.name.toLowerCase() === requestedName.toLowerCase());
    if (existingSecret) {
      const added = await addSecretBinding(existingSecret);
      if (!added) return;
      resetSecretInputs();
      setOpen(false);
      return;
    }

    try {
      const result =
        source === 'internal'
          ? await createInternalSecret.mutateAsync({ data: internalInput } as any)
          : await createExternalSecret.mutateAsync({
              data: normalizeExternalSecretInput(externalInput),
            } as any);

      const added = await addSecretBinding(result.data);
      if (!added) return;
      resetSecretInputs();
      setOpen(false);
      await queryClient.invalidateQueries({ queryKey: ['listSecretDefinitions'] });
    } catch {
      toast.error(
        source === 'internal'
          ? (createInternalSecret.validationErrors ?? 'Failed to create secret')
          : (createExternalSecret.validationErrors ?? 'Failed to create external secret'),
      );
    }
  };

  const testSecret = async () => {
    try {
      const result = await testExternalSecret.mutateAsync({
        data: toExternalSecretTestInput(externalInput),
      } as any);

      if (result.data.success) {
        toast.success(result.data.message);
      } else {
        toast.error(result.data.message);
      }
    } catch {
      toast.error(testExternalSecret.validationErrors ?? 'Failed to test external secret');
    }
  };

  return {
    setOpen,
    dialogProps: {
      open,
      source,
      providers,
      internalInput,
      externalInput,
      isPending: createInternalSecret.isPending || createExternalSecret.isPending,
      isTesting: testExternalSecret.isPending,
      onOpenChange: setOpen,
      onSourceChange: setSource,
      onInternalInputChange: setInternalInput,
      onExternalInputChange: setExternalInput,
      onCreate: createSecret,
      onTest: testSecret,
    },
  };
};

export const normalizeExternalSecretInput = (input: CreateExternalSecretInput): CreateExternalSecretInput => ({
  ...input,
  externalPath: input.externalPath.trim().replace(/^\/+|\/+$/g, ''),
  externalVersion:
    input.externalVersion === '' || input.externalVersion === null ? null : Number(input.externalVersion),
});

export const toExternalSecretInput = (secret: SecretDefinitionView): CreateExternalSecretInput => ({
  name: secret.name,
  providerId: secret.providerId ?? '',
  externalPath: secret.externalPath ?? '',
  externalKey: secret.externalKey ?? '',
  externalVersion: secret.externalVersion ?? null,
});

export const toExternalSecretTestInput = (input: CreateExternalSecretInput): TestExternalSecretInput => {
  const normalized = normalizeExternalSecretInput(input);
  return {
    providerId: normalized.providerId,
    externalPath: normalized.externalPath,
    externalKey: normalized.externalKey,
    externalVersion: normalized.externalVersion,
  };
};

export const CreateSecretDialog = ({
  open,
  source,
  providers,
  internalInput,
  externalInput,
  isPending,
  isTesting,
  onOpenChange,
  onSourceChange,
  onInternalInputChange,
  onExternalInputChange,
  onCreate,
  onTest,
}: {
  open: boolean;
  source: SecretSource;
  providers: SecretProviderView[];
  internalInput: CreateInternalSecretInput;
  externalInput: CreateExternalSecretInput;
  isPending: boolean;
  isTesting: boolean;
  onOpenChange: (open: boolean) => void;
  onSourceChange: (source: SecretSource) => void;
  onInternalInputChange: (
    input: CreateInternalSecretInput | ((prev: CreateInternalSecretInput) => CreateInternalSecretInput),
  ) => void;
  onExternalInputChange: (
    input: CreateExternalSecretInput | ((prev: CreateExternalSecretInput) => CreateExternalSecretInput),
  ) => void;
  onCreate: () => void;
  onTest: () => void;
}) => {
  const canCreate =
    source === 'internal'
      ? Boolean(internalInput.name.trim())
      : Boolean(
          externalInput.name.trim() &&
          externalInput.providerId &&
          externalInput.externalPath.trim() &&
          externalInput.externalKey.trim(),
        );

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-137.5" onInteractOutside={(e) => e.preventDefault()}>
        <DialogHeader>
          <DialogTitle>Create Stored Secret</DialogTitle>
          <DialogDescription>
            Internal secrets are stored encrypted by Citadel. Vault secrets store only the provider reference and are
            resolved during apply.
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-4">
          <label className="grid gap-1.5 text-sm">
            <span className="text-xs text-muted-foreground">Source</span>
            <Select value={source} onValueChange={(value) => onSourceChange(value as SecretSource)}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="internal">Internal encrypted</SelectItem>
                <SelectItem value="vault">Vault-compatible KV v2</SelectItem>
              </SelectContent>
            </Select>
          </label>

          {source === 'internal' ? (
            <>
              <LabeledInput
                label="Name"
                value={internalInput.name}
                onChange={(name) => onInternalInputChange((prev) => ({ ...prev, name }))}
                placeholder="API_KEY"
                description="Default runtime key used when this secret is added to a resource."
              />
              <LabeledInput
                label="Value"
                type="password"
                value={internalInput.value}
                onChange={(value) => onInternalInputChange((prev) => ({ ...prev, value }))}
                placeholder="Secret value"
                description="Stored encrypted and redacted from logs."
              />
            </>
          ) : (
            <>
              <LabeledInput
                label="Name"
                value={externalInput.name}
                onChange={(name) => onExternalInputChange((prev) => ({ ...prev, name }))}
                placeholder="API_KEY"
                description="Default runtime key used when this secret is added to a resource."
              />
              <div className="grid gap-1.5 text-sm">
                <span className="text-xs text-muted-foreground">Provider</span>
                <Select
                  value={externalInput.providerId}
                  disabled={providers.length === 0}
                  onValueChange={(providerId) => onExternalInputChange((prev) => ({ ...prev, providerId }))}>
                  <SelectTrigger>
                    <SelectValue placeholder={providers.length ? 'Select provider' : 'No providers'} />
                  </SelectTrigger>
                  <SelectContent>
                    {providers.map((provider) => (
                      <SelectItem key={provider.id} value={provider.id}>
                        {provider.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <span className="text-xs text-muted-foreground">
                  Create providers in the Secret Providers section. The secret value is fetched only during apply.
                </span>
              </div>
              <LabeledInput
                label="Path"
                value={externalInput.externalPath}
                onChange={(externalPath) => onExternalInputChange((prev) => ({ ...prev, externalPath }))}
                placeholder="apps/api/prod"
                description="Path inside the selected mount. Do not include the mount name, /v1, or /data."
              />
              <LabeledInput
                label="Key"
                value={externalInput.externalKey}
                onChange={(externalKey) => onExternalInputChange((prev) => ({ ...prev, externalKey }))}
                placeholder="api_key"
                description="Field name inside the Vault secret data object."
              />
              <LabeledInput
                label="Version"
                type="number"
                value={externalInput.externalVersion?.toString() ?? ''}
                onChange={(value) =>
                  onExternalInputChange((prev) => ({ ...prev, externalVersion: value ? Number(value) : null }))
                }
                placeholder="Latest"
                description="Leave empty to resolve the latest version."
              />
            </>
          )}
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          {source === 'vault' && (
            <Button type="button" variant="outline" onClick={onTest} disabled={!canCreate || isPending || isTesting}>
              Test {isTesting && <LoaderCircle className="ml-1 h-3.5 w-3.5 animate-spin" />}
            </Button>
          )}
          <Button type="button" onClick={onCreate} disabled={!canCreate || isPending}>
            Create {isPending && <LoaderCircle className="ml-1 h-3.5 w-3.5 animate-spin" />}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export const toEditableEntry = (entry: ResourceBindingView): EditableEntry => ({
  clientId: entry.id,
  name: entry.name,
  kind: entry.kind,
  value: entry.value,
  secretId: entry.secretId,
  secretDeliveryMode: entry.secretDeliveryMode,
  targetPath: entry.targetPath,
});

export const toInput = (entry: EditableEntry): ResourceBindingInput => ({
  name: entry.name,
  kind: entry.kind,
  value: entry.kind === ResourceBindingKind.Variable ? (entry.value ?? '') : null,
  secretId: entry.kind === ResourceBindingKind.Secret ? entry.secretId : null,
  secretDeliveryMode:
    entry.kind === ResourceBindingKind.Secret ? (entry.secretDeliveryMode ?? ENV_DELIVERY_MODE) : null,
  targetPath: entry.kind === ResourceBindingKind.Secret ? (entry.targetPath ?? null) : null,
});

const newVariableInput = (): ResourceBindingInput => ({
  name: '',
  kind: ResourceBindingKind.Variable,
  value: '',
  secretId: null,
  secretDeliveryMode: null,
  targetPath: null,
});

const newSecretInput = (secret?: SecretDefinitionView): ResourceBindingInput => ({
  name: secret?.name ?? '',
  kind: ResourceBindingKind.Secret,
  value: null,
  secretId: secret?.id ?? null,
  secretDeliveryMode: ENV_DELIVERY_MODE,
  targetPath: null,
});

const normalizeEntryInput = (entry: ResourceBindingInput): ResourceBindingInput => ({
  name: entry.name,
  kind: entry.kind,
  value: entry.kind === ResourceBindingKind.Variable ? (entry.value ?? '') : null,
  secretId: entry.kind === ResourceBindingKind.Secret ? entry.secretId : null,
  secretDeliveryMode:
    entry.kind === ResourceBindingKind.Secret ? (entry.secretDeliveryMode ?? ENV_DELIVERY_MODE) : null,
  targetPath: entry.kind === ResourceBindingKind.Secret ? (entry.targetPath ?? null) : null,
});

const toUpdateInput = (id: string, entry: ResourceBindingInput): UpdateResourceBindingInput => ({
  id,
  ...normalizeEntryInput(entry),
});

const hasDuplicateEntryName = (entries: ResourceBindingInput[]): boolean => {
  const names = new Set<string>();
  for (const entry of entries) {
    const name = entry.name.trim().toLowerCase();
    if (names.has(name)) return true;
    names.add(name);
  }

  return false;
};

export const toInputFromView = (entry: ResourceBindingView): ResourceBindingInput => ({
  name: entry.name,
  kind: entry.kind,
  value: entry.kind === ResourceBindingKind.Variable ? (entry.value ?? '') : null,
  secretId: entry.kind === ResourceBindingKind.Secret ? entry.secretId : null,
  secretDeliveryMode:
    entry.kind === ResourceBindingKind.Secret ? (entry.secretDeliveryMode ?? ENV_DELIVERY_MODE) : null,
  targetPath: entry.kind === ResourceBindingKind.Secret ? (entry.targetPath ?? null) : null,
});
