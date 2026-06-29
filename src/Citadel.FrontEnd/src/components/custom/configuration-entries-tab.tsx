import {
  ConfigurationEntryInput,
  ConfigurationEntryKind,
  ConfigurationEntryView,
  ConfigurationScope,
  CreateExternalSecretInput,
  CreateInternalSecretInput,
  TestExternalSecretInput,
  CreateVaultKvV2SecretProviderInput,
  SecretDefinitionView,
  SecretProviderView,
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
import { ChevronDown, ExternalLink, KeyRound, LoaderCircle, MoreHorizontal, Pencil, Plus, Trash2, Variable } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import { FieldInput } from './form-builder';
import { Link } from 'react-router';

export const ENV_DELIVERY_MODE = 'EnvironmentVariable';
const INTERNAL_SECRET_INPUT: CreateInternalSecretInput = { name: '', value: '' };
const EXTERNAL_SECRET_INPUT: CreateExternalSecretInput = {
  name: '',
  providerId: '',
  externalPath: '',
  externalKey: '',
  externalVersion: null,
};
const PROVIDER_INPUT: CreateVaultKvV2SecretProviderInput = {
  name: '',
  address: '',
  mountPath: 'secret',
  token: '',
};
type SecretSource = 'internal' | 'vault';

export type EditableEntry = ConfigurationEntryInput & {
  clientId: string;
};

const EMPTY_CONFIGURATION_ENTRIES: ConfigurationEntryView[] = [];
const EMPTY_SECRET_DEFINITIONS: SecretDefinitionView[] = [];

export const ConfigurationSummary = ({
  scope,
  resourceId,
  title = 'Variables',
  description = 'Resource-specific values are managed in the Variables tab. Global values are resolved during deploy.',
}: {
  scope: ConfigurationScope.Stack | ConfigurationScope.Deployment;
  resourceId: string;
  title?: string;
  description?: string;
}) => {
  const args = useMemo(() => ({ scope, resourceId }), [resourceId, scope]);
  const { data, isLoading } = useRead('getResourceConfigurationEntries', args);
  const effectiveEntries = data?.data.effectiveEntries ?? [];
  const variableCount = effectiveEntries.filter((entry) => entry.kind === ConfigurationEntryKind.Variable).length;
  const secretCount = effectiveEntries.filter((entry) => entry.kind === ConfigurationEntryKind.Secret).length;

  const openEnvironmentTab = () => {
    window.location.hash = 'variables';
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
            Open Variables
          </Button>
        </div>
      </div>
    </div>
  );
};

export const ConfigurationEntriesTab = ({
  scope,
  resourceId,
  disabled,
}: {
  scope: ConfigurationScope.Stack | ConfigurationScope.Deployment;
  resourceId: string;
  disabled?: boolean;
}) => {
  const args = useMemo(() => ({ scope, resourceId }), [resourceId, scope]);
  const secretQueryArgs = useMemo(() => ({ query: { scope, resourceId } }), [resourceId, scope]);
  const { data, isLoading } = useRead('getResourceConfigurationEntries', args);
  const { data: secretsData } = useRead('listSecretDefinitions', secretQueryArgs);
  const serverEntries = data?.data.entries ?? EMPTY_CONFIGURATION_ENTRIES;
  const secrets = useMemo(() => secretsData?.data.secrets ?? EMPTY_SECRET_DEFINITIONS, [secretsData?.data.secrets]);
  const canCreateSecret = Boolean(secretsData?.data.capabilities.canWrite);
  const initialEntries = useMemo(() => serverEntries.map(toEditableEntry), [serverEntries]);
  const originalInputs = useMemo(() => serverEntries.map(toInputFromView), [serverEntries]);
  const resetKey = useMemo(() => JSON.stringify(originalInputs), [originalInputs]);

  return (
    <ConfigurationEntriesTabEditor
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
    />
  );
};

const ConfigurationEntriesTabEditor = ({
  scope,
  resourceId,
  queryArgs,
  disabled,
  isLoading,
  initialEntries,
  originalInputs,
  secrets,
  canCreateSecret,
}: {
  scope: ConfigurationScope.Stack | ConfigurationScope.Deployment;
  resourceId: string;
  queryArgs: { scope: ConfigurationScope.Stack | ConfigurationScope.Deployment; resourceId: string };
  disabled?: boolean;
  isLoading: boolean;
  initialEntries: EditableEntry[];
  originalInputs: ConfigurationEntryInput[];
  secrets: SecretDefinitionView[];
  canCreateSecret: boolean;
}) => {
  const queryClient = useQueryClient();
  const replace = useMutate('replaceResourceConfigurationEntries');
  const entries = initialEntries;
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingEntry, setEditingEntry] = useState<EditableEntry | null>(null);
  const [entryInput, setEntryInput] = useState<ConfigurationEntryInput>(newVariableInput());
  const [deleteEntry, setDeleteEntry] = useState<EditableEntry | null>(null);

  const saveEntries = async (entriesToSave: ConfigurationEntryInput[], message: string) => {
    try {
      await replace.mutateAsync({
        scope,
        resourceId,
        data: { entries: entriesToSave },
      } as any);
      await queryClient.invalidateQueries({ queryKey: ['getResourceConfigurationEntries', queryArgs] });
      toast.success(message);
    } catch {
      toast.error(replace.validationErrors ?? 'Failed to save variables and secrets');
    }
  };

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

  const addSecretBinding = useCallback(
    async (secret?: SecretDefinitionView) => {
      if (!secret) return;
      await saveEntries([...originalInputs, newSecretInput(secret)], 'Secret created and added');
    },
    [originalInputs],
  );
  const secretCreation = useSecretCreation(addSecretBinding);

  const saveDialogEntry = async () => {
    const next =
      editingEntry == null
        ? [...originalInputs, normalizeEntryInput(entryInput)]
        : originalInputs.map((entry, index) =>
            entries[index].clientId === editingEntry.clientId ? normalizeEntryInput(entryInput) : entry,
          );

    await saveEntries(next, editingEntry == null ? 'Configuration entry added' : 'Configuration entry updated');
    setDialogOpen(false);
    setEditingEntry(null);
  };

  const confirmDelete = async () => {
    if (!deleteEntry) return;
    await saveEntries(
      entries.filter((entry) => entry.clientId !== deleteEntry.clientId).map(toInput),
      'Configuration entry deleted',
    );
    setDeleteEntry(null);
  };

  return (
    <div className="flex w-full flex-col gap-4">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
        <div className="text-sm text-muted-foreground">
          Define resource-specific variables and secrets. Values here override global Variables entries with the same
          name.
        </div>
        <div className="flex flex-wrap gap-2">
          <ConfigurationAddDropdown
            disabled={disabled}
            hasSecrets={secrets.length > 0}
            canCreateSecret={canCreateSecret}
            onAddVariable={openAddVariable}
            onBindSecret={openAddSecret}
            onCreateSecret={() => secretCreation.setOpen(true)}
          />
        </div>
      </div>

      <ResourceConfigurationEntriesTable
        entries={entries}
        secrets={secrets}
        isLoading={isLoading}
        emptyText="No resource variables or secrets are defined."
        disabled={disabled}
        onEdit={openEdit}
        onDelete={setDeleteEntry}
      />

      <GlobalVariablesLink />

      <ConfigurationEntryDialog
        open={dialogOpen}
        input={entryInput}
        secrets={secrets}
        isPending={replace.isPending}
        editing={editingEntry != null}
        onOpenChange={setDialogOpen}
        onInputChange={setEntryInput}
        onSave={saveDialogEntry}
      />
      <CreateSecretDialog {...secretCreation.dialogProps} />
      <Dialog open={deleteEntry != null} onOpenChange={(open) => !open && setDeleteEntry(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete configuration entry</DialogTitle>
            <DialogDescription>
              Delete `{deleteEntry?.name}` from this resource. Global entries with the same name will become effective
              again.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteEntry(null)} disabled={replace.isPending}>
              Cancel
            </Button>
            <Button variant="destructive" onClick={confirmDelete} disabled={replace.isPending}>
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
};

const ResourceConfigurationEntriesTable = ({
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
                    entry.kind === ConfigurationEntryKind.Secret ? 'border-dashed' : '',
                  )}>
                  {entry.kind === ConfigurationEntryKind.Secret ? (
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
                {entry.kind === ConfigurationEntryKind.Variable ? (
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
                <span className="text-xs text-muted-foreground">
                  {entry.kind === ConfigurationEntryKind.Secret ? (entry.secretDeliveryMode ?? ENV_DELIVERY_MODE) : '-'}
                </span>
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

const GlobalVariablesLink = () => (
  <div className="flex items-center justify-between rounded-sm border border-dashed px-4 py-3 text-sm">
    <div>
      <div className="font-medium">Global Variables</div>
      <div className="text-xs text-muted-foreground">
        Global entries are inherited automatically and resolved during deploy. They are not listed in this resource table.
      </div>
    </div>
    <Button asChild variant="outline" size="sm">
      <Link to="/variables">
        <ExternalLink className="size-3.5" />
        Open Variables
      </Link>
    </Button>
  </div>
);

const ConfigurationEntryDialog = ({
  open,
  input,
  secrets,
  isPending,
  editing,
  onOpenChange,
  onInputChange,
  onSave,
}: {
  open: boolean;
  input: ConfigurationEntryInput;
  secrets: SecretDefinitionView[];
  isPending: boolean;
  editing: boolean;
  onOpenChange: (open: boolean) => void;
  onInputChange: (input: ConfigurationEntryInput | ((prev: ConfigurationEntryInput) => ConfigurationEntryInput)) => void;
  onSave: () => void;
}) => {
  const isSecret = input.kind === ConfigurationEntryKind.Secret;
  const canSave = input.name.trim().length > 0 && (!isSecret || Boolean(input.secretId));

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
            <div>
              <Label>Secret</Label>
              <Select
                value={input.secretId ?? ''}
                disabled={secrets.length === 0}
                onValueChange={(secretId) => onInputChange((prev) => ({ ...prev, secretId }))}>
                <SelectTrigger>
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

export const ConfigurationAddDropdown = ({
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

export const useSecretCreation = (addSecretBinding: (secret?: SecretDefinitionView) => void | Promise<void>) => {
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
    try {
      const result =
        source === 'internal'
          ? await createInternalSecret.mutateAsync({ data: internalInput } as any)
          : await createExternalSecret.mutateAsync({
              data: normalizeExternalSecretInput(externalInput),
            } as any);

      await addSecretBinding(result.data);
      resetSecretInputs();
      setOpen(false);
      await queryClient.invalidateQueries({ queryKey: ['listSecretDefinitions'] });
      toast.success('Secret created');
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

const normalizeExternalSecretInput = (input: CreateExternalSecretInput): CreateExternalSecretInput => ({
  ...input,
  externalPath: input.externalPath.trim().replace(/^\/+|\/+$/g, ''),
  externalVersion: input.externalVersion === '' || input.externalVersion === null ? null : Number(input.externalVersion),
});

const toExternalSecretTestInput = (input: CreateExternalSecretInput): TestExternalSecretInput => {
  const normalized = normalizeExternalSecretInput(input);
  return {
    providerId: normalized.providerId,
    externalPath: normalized.externalPath,
    externalKey: normalized.externalKey,
    externalVersion: normalized.externalVersion,
  };
};

export const SecretProvidersSection = ({ disabled }: { disabled?: boolean }) => {
  const queryClient = useQueryClient();
  const { data, isLoading } = useRead('listSecretProviders');
  const createProvider = useMutate('createVaultKvV2SecretProvider');
  const providers = data?.data.providers ?? [];
  const [open, setOpen] = useState(false);
  const [input, setInput] = useState<CreateVaultKvV2SecretProviderInput>(PROVIDER_INPUT);

  const openAdd = () => {
    setInput(PROVIDER_INPUT);
    setOpen(true);
  };

  const save = async () => {
    try {
      await createProvider.mutateAsync({ data: input } as any);
      setInput(PROVIDER_INPUT);
      setOpen(false);
      await queryClient.invalidateQueries({ queryKey: ['listSecretProviders', {}] });
      toast.success('Secret provider created');
    } catch {
      toast.error(createProvider.validationErrors ?? 'Failed to create secret provider');
    }
  };

  return (
    <div className="flex flex-col gap-6">
      <div className="flex justify-between">
        <div className="flex items-center gap-3">
          <div className="inline-flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10 text-primary">
            <KeyRound className="h-4 w-4" />
          </div>
          <div>
            <div className="font-bold">Secret Providers</div>
            <p className="text-xs text-muted-foreground">External providers used by Vault-backed secret definitions.</p>
          </div>
        </div>
        <Button variant="outline" disabled={disabled} onClick={openAdd}>
          <Plus className="h-3 w-3" /> Create Vault Provider
        </Button>
      </div>

      <div className="grid grid-cols-1 gap-4 md:grid-cols-3 lg:grid-cols-4">
        {providers.map((provider) => (
          <SecretProviderCard key={provider.id} provider={provider} />
        ))}

        {!isLoading && (
          <button
            type="button"
            onClick={disabled ? undefined : openAdd}
            disabled={disabled}
            className={cn(
              'flex h-35 flex-col items-center justify-center gap-3 rounded-xl border border-dashed p-4 text-zinc-400 hover:text-zinc-600',
              disabled && 'hover:cursor-not-allowed opacity-70',
            )}>
            <Plus className="h-5 w-5" />
            <span className="text-sm font-medium">Create Vault Provider</span>
          </button>
        )}
      </div>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="sm:max-w-137.5" onInteractOutside={(e) => e.preventDefault()}>
          <DialogHeader>
            <DialogTitle>Create Vault Provider</DialogTitle>
            <DialogDescription>Configure a Vault-compatible KV v2 endpoint used by external secrets.</DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-4">
            <div>
              <Label>Name</Label>
              <FieldInput
                className="max-w-full"
                value={input.name}
                placeholder="Production Vault"
                onChange={(name) => setInput((prev) => ({ ...prev, name }))}
              />
            </div>
            <div>
              <Label>Address</Label>
              <FieldInput
                className="max-w-full"
                value={input.address}
                placeholder="https://vault.example.com"
                onChange={(address) => setInput((prev) => ({ ...prev, address }))}
              />
              <div className="mt-1 text-xs text-muted-foreground">
                Base Vault URL. Citadel appends the KV v2 API path.
              </div>
            </div>
            <div>
              <Label>Mount Path</Label>
              <FieldInput
                className="max-w-full"
                value={input.mountPath}
                placeholder="secret"
                onChange={(mountPath) => setInput((prev) => ({ ...prev, mountPath }))}
              />
              <div className="mt-1 text-xs text-muted-foreground">KV v2 mount name, not the secret path.</div>
            </div>
            <div>
              <Label>Token</Label>
              <FieldInput
                className="max-w-full"
                type="password"
                value={input.token}
                placeholder="Vault token"
                onChange={(token) => setInput((prev) => ({ ...prev, token }))}
              />
              <div className="mt-1 text-xs text-muted-foreground">
                Stored encrypted and used only when resolving external secrets.
              </div>
            </div>
          </div>

          <DialogFooter className="flex w-full justify-end items-center">
            <div className="flex gap-2">
              <Button variant="outline" onClick={() => setOpen(false)} disabled={createProvider.isPending}>
                Cancel
              </Button>
              <Button
                onClick={save}
                disabled={
                  createProvider.isPending ||
                  !input.name.trim() ||
                  !input.address.trim() ||
                  !input.mountPath.trim() ||
                  !input.token.trim()
                }>
                Save {createProvider.isPending && <LoaderCircle className="ml-1 h-3.5 w-3.5 animate-spin" />}
              </Button>
            </div>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
};

const SecretProviderCard = ({ provider }: { provider: SecretProviderView }) => {
  const letter = provider.name.charAt(0).toUpperCase();

  return (
    <div className="group relative flex h-35 flex-col justify-between rounded-xl border border-muted bg-background p-4 transition-all hover:border-zinc-300 hover:shadow-md">
      <div className="flex items-start justify-between">
        <div className="flex min-w-0 items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-lg border border-violet-600/10 bg-violet-600 font-bold text-white shadow-sm">
            {letter}
          </div>
          <div className="min-w-0">
            <h3 className="truncate text-sm font-semibold">{provider.name}</h3>
            <p className="truncate text-xs text-muted-foreground">{provider.address}</p>
          </div>
        </div>
      </div>

      <div className="mt-4 flex items-center justify-between border-t border-muted pt-4">
        <span className="text-xs font-medium text-zinc-600">Vault KV v2</span>
        <span className="rounded-sm border px-2 py-0.5 text-xs text-muted-foreground">{provider.mountPath}</span>
      </div>
    </div>
  );
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
                description="KV v2 secret path under the selected mount."
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

export const toEditableEntry = (entry: ConfigurationEntryView): EditableEntry => ({
  clientId: entry.id,
  name: entry.name,
  kind: entry.kind,
  value: entry.value,
  secretId: entry.secretId,
  secretDeliveryMode: entry.secretDeliveryMode,
  targetPath: entry.targetPath,
});

export const toInput = (entry: EditableEntry): ConfigurationEntryInput => ({
  name: entry.name,
  kind: entry.kind,
  value: entry.kind === ConfigurationEntryKind.Variable ? (entry.value ?? '') : null,
  secretId: entry.kind === ConfigurationEntryKind.Secret ? entry.secretId : null,
  secretDeliveryMode:
    entry.kind === ConfigurationEntryKind.Secret ? (entry.secretDeliveryMode ?? ENV_DELIVERY_MODE) : null,
  targetPath: entry.kind === ConfigurationEntryKind.Secret ? (entry.targetPath ?? null) : null,
});

const newVariableInput = (): ConfigurationEntryInput => ({
  name: '',
  kind: ConfigurationEntryKind.Variable,
  value: '',
  secretId: null,
  secretDeliveryMode: null,
  targetPath: null,
});

const newSecretInput = (secret?: SecretDefinitionView): ConfigurationEntryInput => ({
  name: secret?.name ?? '',
  kind: ConfigurationEntryKind.Secret,
  value: null,
  secretId: secret?.id ?? null,
  secretDeliveryMode: ENV_DELIVERY_MODE,
  targetPath: null,
});

const normalizeEntryInput = (entry: ConfigurationEntryInput): ConfigurationEntryInput => ({
  name: entry.name,
  kind: entry.kind,
  value: entry.kind === ConfigurationEntryKind.Variable ? (entry.value ?? '') : null,
  secretId: entry.kind === ConfigurationEntryKind.Secret ? entry.secretId : null,
  secretDeliveryMode:
    entry.kind === ConfigurationEntryKind.Secret ? (entry.secretDeliveryMode ?? ENV_DELIVERY_MODE) : null,
  targetPath: entry.kind === ConfigurationEntryKind.Secret ? (entry.targetPath ?? null) : null,
});

export const toInputFromView = (entry: ConfigurationEntryView): ConfigurationEntryInput => ({
  name: entry.name,
  kind: entry.kind,
  value: entry.kind === ConfigurationEntryKind.Variable ? (entry.value ?? '') : null,
  secretId: entry.kind === ConfigurationEntryKind.Secret ? entry.secretId : null,
  secretDeliveryMode:
    entry.kind === ConfigurationEntryKind.Secret ? (entry.secretDeliveryMode ?? ENV_DELIVERY_MODE) : null,
  targetPath: entry.kind === ConfigurationEntryKind.Secret ? (entry.targetPath ?? null) : null,
});
