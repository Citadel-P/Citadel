import {
  CreateExternalSecretInput,
  ResourceBindingInput,
  ResourceBindingKind,
  ResourceBindingView,
  SecretDefinitionView,
  SecretProviderType,
  UpdateExternalSecretInput,
  UpdateResourceBindingInput,
} from '@/api/generated/api.types';
import {
  ResourceBindingAddDropdown,
  CreateSecretDialog,
  EditExternalSecretDialog,
  ENV_DELIVERY_MODE,
  normalizeExternalSecretInput,
  toInputFromView,
  toExternalSecretInput,
  toExternalSecretTestInput,
  useSecretCreation,
} from '@/components/custom/resource-bindings-tab';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { useMutate, useRead } from '@/lib/hooks';
import { ActionData } from '@/pages/types';
import { useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { toast } from 'sonner';
import { ResourceBindingsDataTable } from '@/components/custom/resource-bindings-table';
import {
  BindingDropdownActions,
  BindingPageAction,
  dispatchBindingPageAction,
  invalidateBindingQueries,
  subscribeBindingPageActions,
} from './actions';

type EntryDialogMode = 'add-variable' | 'add-secret-key' | 'edit';

const EMPTY_ENTRIES: ResourceBindingView[] = [];
const EMPTY_SECRETS: SecretDefinitionView[] = [];
const EXTERNAL_SECRET_INPUT: CreateExternalSecretInput = {
  name: '',
  providerId: '',
  externalPath: '',
  externalKey: '',
  externalVersion: null,
};

export const BindingsAddButton = () => {
  const { data: configData } = useRead('getGlobalResourceBindings');
  const { data: secretsData } = useRead('listSecretDefinitions');
  const secrets = useMemo(() => secretsData?.data.secrets ?? EMPTY_SECRETS, [secretsData?.data.secrets]);
  const canWrite = Boolean(configData?.data.capabilities?.canWrite);
  const canCreateSecret = Boolean(secretsData?.data.capabilities.canWrite);

  return (
    <ResourceBindingAddDropdown
      disabled={!canWrite}
      hasSecrets={secrets.length > 0}
      canCreateSecret={canCreateSecret}
      onAddVariable={() => dispatchBindingPageAction({ type: 'add-variable' })}
      onBindSecret={() => dispatchBindingPageAction({ type: 'add-secret-key' })}
      onCreateSecret={() => dispatchBindingPageAction({ type: 'create-secret' })}
    />
  );
};

export const BindingsTable = ({
  items,
  isLoading,
  actions,
}: {
  items: ResourceBindingView[];
  isLoading: boolean;
  isFiltered?: boolean;
  actions?: Record<
    string,
    React.FC<{ resource: ResourceBindingView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) => {
  const queryClient = useQueryClient();
  const { data } = useRead('getGlobalResourceBindings');
  const { data: secretsData } = useRead('listSecretDefinitions');
  const create = useMutate('createGlobalResourceBinding');
  const update = useMutate('updateGlobalResourceBinding');
  const updateExternalSecret = useMutate('updateExternalSecret');
  const testExternalSecret = useMutate('testExternalSecret');
  const allEntries = data?.data.entries ?? EMPTY_ENTRIES;
  const canWrite = Boolean(data?.data.capabilities?.canWrite);
  const canCreateSecret = Boolean(secretsData?.data.capabilities.canWrite);
  const originalInputs = useMemo(() => allEntries.map(toInputFromView), [allEntries]);
  const secrets = useMemo(() => secretsData?.data.secrets ?? EMPTY_SECRETS, [secretsData?.data.secrets]);
  const [dialogMode, setDialogMode] = useState<EntryDialogMode>('add-variable');
  const [editingEntry, setEditingEntry] = useState<ResourceBindingView | null>(null);
  const [entryInput, setEntryInput] = useState<ResourceBindingInput>(newVariableInput());
  const [entryDialogOpen, setEntryDialogOpen] = useState(false);
  const [editingStoredSecret, setEditingStoredSecret] = useState<SecretDefinitionView | null>(null);
  const [storedSecretInput, setStoredSecretInput] = useState<CreateExternalSecretInput>(EXTERNAL_SECRET_INPUT);
  const { data: providersData } = useRead('listSecretProviders', undefined, { enabled: editingStoredSecret != null });
  const providers = useMemo(() => providersData?.data.providers ?? [], [providersData?.data.providers]);

  const updateEntry = useCallback(
    async (entryId: string, entry: ResourceBindingInput, successMessage?: string) => {
      await update.mutateAsync({ data: toUpdateInput(entryId, entry) } as any);
      await invalidateBindingQueries(queryClient);
      if (successMessage) {
        toast.success(successMessage);
      }
    },
    [queryClient, update],
  );

  const createEntry = useCallback(
    async (entry: ResourceBindingInput, successMessage?: string) => {
      await create.mutateAsync({ data: entry } as any);
      await invalidateBindingQueries(queryClient);
      if (successMessage) {
        toast.success(successMessage);
      }
    },
    [create, queryClient],
  );

  const openEntryDialog = useCallback(
    (mode: EntryDialogMode, entry?: ResourceBindingView) => {
      if (!canWrite) return;

      setDialogMode(mode);
      setEditingEntry(entry ?? null);
      setEntryInput(getInitialEntryInput(mode, secrets, entry));
      setEntryDialogOpen(true);
    },
    [canWrite, secrets],
  );

  const secretCreation = useSecretCreation(
    useCallback(
      async (secret?: SecretDefinitionView) => {
        if (!secret) return false;
        const next = [...originalInputs, newSecretBindingInput(secret)];
        if (hasDuplicateEntryName(next)) {
          toast.error('A variable or secret key with this name already exists.');
          return false;
        }

        await createEntry(newSecretBindingInput(secret), 'Secret key added');
        return true;
      },
      [createEntry, originalInputs],
    ),
    secrets,
  );
  const setCreateSecretOpen = secretCreation.setOpen;
  const openCreateSecretDialog = useCallback(
    (open: boolean) => {
      if (open && !canCreateSecret) {
        toast.error('Binding write permission is required to create secrets');
        return;
      }

      setCreateSecretOpen(open);
    },
    [canCreateSecret, setCreateSecretOpen],
  );

  useEffect(() => {
    return subscribeBindingPageActions((action: BindingPageAction) => {
      if (!canWrite) return;

      if (action.type === 'add-variable') openEntryDialog('add-variable');
      if (action.type === 'add-secret-key') openEntryDialog('add-secret-key');
      if (action.type === 'create-secret') openCreateSecretDialog(true);
      if (action.type === 'edit-entry') openEntryDialog('edit', action.entry);
    });
  }, [canWrite, openCreateSecretDialog, openEntryDialog]);

  const saveEntry = async () => {
    if (!canWrite) return;

    try {
      const next =
        dialogMode === 'edit' && editingEntry
          ? originalInputs.map((entry, index) => (allEntries[index].id === editingEntry.id ? normalizeInput(entryInput) : entry))
          : [...originalInputs, normalizeInput(entryInput)];

      if (hasDuplicateEntryName(next)) {
        toast.error('A variable or secret key with this name already exists.');
        return;
      }

      if (dialogMode === 'edit') {
        await updateEntry(editingEntry!.id, normalizeInput(entryInput), 'Entry updated');
      } else {
        await createEntry(normalizeInput(entryInput), 'Entry created');
      }
      setEntryDialogOpen(false);
      setEditingEntry(null);
    } catch {
      toast.error((dialogMode === 'edit' ? update.validationErrors : create.validationErrors) ?? 'Failed to save entry');
    }
  };

  const openEditStoredSecret = (secret: SecretDefinitionView) => {
    if (secret.providerType !== SecretProviderType.VaultCompatibleKvV2) {
      toast.error('Only Vault-compatible stored secrets can be edited here.');
      return;
    }

    setEditingStoredSecret(secret);
    setStoredSecretInput(toExternalSecretInput(secret));
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
      await invalidateBindingQueries(queryClient);
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
    <div className="flex w-full flex-col gap-6">
      <ResourceBindingsDataTable
        items={items ?? EMPTY_ENTRIES}
        secrets={secrets}
        isLoading={isLoading}
        actions={actions ?? BindingDropdownActions}
      />

      <EntryEditorDialog
        open={entryDialogOpen}
        mode={dialogMode}
        input={entryInput}
        secrets={secrets}
        isPending={update.isPending || create.isPending}
        onOpenChange={setEntryDialogOpen}
        onInputChange={setEntryInput}
        onSave={saveEntry}
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
    </div>
  );
};

const EntryEditorDialog = ({
  open,
  mode,
  input,
  secrets,
  isPending,
  onOpenChange,
  onInputChange,
  onSave,
  onEditStoredSecret,
}: {
  open: boolean;
  mode: EntryDialogMode;
  input: ResourceBindingInput;
  secrets: SecretDefinitionView[];
  isPending: boolean;
  onOpenChange: (open: boolean) => void;
  onInputChange: (input: ResourceBindingInput | ((prev: ResourceBindingInput) => ResourceBindingInput)) => void;
  onSave: () => void;
  onEditStoredSecret?: (secret: SecretDefinitionView) => void;
}) => {
  const isSecret = input.kind === ResourceBindingKind.Secret;
  const selectedSecret = isSecret ? secrets.find((secret) => secret.id === input.secretId) : undefined;
  const canSave = input.name.trim().length > 0 && (!isSecret || Boolean(input.secretId));

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-137.5" onInteractOutside={(event) => event.preventDefault()}>
        <DialogHeader>
          <DialogTitle>{getDialogTitle(mode)}</DialogTitle>
          <DialogDescription>
            {isSecret
              ? 'Expose a runtime key backed by a stored secret. The secret value is resolved only during apply.'
              : 'Create or update a global variable inherited by stacks and deployments.'}
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
            <div className="mt-1 text-xs text-muted-foreground">
              Environment key available to compose interpolation.
            </div>
          </div>

          {isSecret ? (
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
                  <SelectContent className='bg-background'>
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
              <div className="mt-1 text-xs text-muted-foreground">
                The selected secret is redacted in logs and resolved during deployment.
              </div>
            </div>
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

const getDialogTitle = (mode: EntryDialogMode) => {
  if (mode === 'edit') return 'Edit Entry';
  if (mode === 'add-secret-key') return 'Add Secret Key';
  return 'Add Variable';
};

const getInitialEntryInput = (
  mode: EntryDialogMode,
  secrets: SecretDefinitionView[],
  entry?: ResourceBindingView,
): ResourceBindingInput => {
  if (entry) return toInputFromView(entry);
  if (mode === 'add-secret-key') return newSecretBindingInput(secrets[0]);
  return newVariableInput();
};

const newVariableInput = (): ResourceBindingInput => ({
  name: '',
  kind: ResourceBindingKind.Variable,
  value: '',
  secretId: null,
  secretDeliveryMode: null,
  targetPath: null,
});

const newSecretBindingInput = (secret?: SecretDefinitionView): ResourceBindingInput => ({
  name: secret?.name ?? '',
  kind: ResourceBindingKind.Secret,
  value: null,
  secretId: secret?.id ?? null,
  secretDeliveryMode: ENV_DELIVERY_MODE,
  targetPath: null,
});

const normalizeInput = (entry: ResourceBindingInput): ResourceBindingInput => ({
  name: entry.name,
  kind: entry.kind,
  value: entry.kind === ResourceBindingKind.Variable ? (entry.value ?? '') : null,
  secretId: entry.kind === ResourceBindingKind.Secret ? entry.secretId : null,
  secretDeliveryMode: entry.kind === ResourceBindingKind.Secret ? (entry.secretDeliveryMode ?? ENV_DELIVERY_MODE) : null,
  targetPath: entry.kind === ResourceBindingKind.Secret ? (entry.targetPath ?? null) : null,
});

const toUpdateInput = (id: string, entry: ResourceBindingInput): UpdateResourceBindingInput => ({
  id,
  ...normalizeInput(entry),
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

