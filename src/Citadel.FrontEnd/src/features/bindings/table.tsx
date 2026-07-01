import {
  ResourceBindingInput,
  ResourceBindingKind,
  ResourceBindingView,
  SecretDefinitionView,
} from '@/api/generated/api.types';
import {
  ResourceBindingAddDropdown,
  CreateSecretDialog,
  ENV_DELIVERY_MODE,
  toInputFromView,
  useSecretCreation,
} from '@/components/custom/resource-bindings-tab';
import { ActionBar } from '@/components/custom/action-bar';
import { ConfirmDeleteDialog } from '@/components/custom/confirm-delete-dialog';
import { ContentCard } from '@/components/custom/content-card';
import SortableCell from '@/components/custom/sortable-cell';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { DataTable } from '@/components/ui/data-table';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
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
import { useSelectedResources } from '@/lib/atoms';
import { useMutate, useRead } from '@/lib/hooks';
import { ActionData, ButtonGroupComponent } from '@/pages/types';
import { useQueryClient } from '@tanstack/react-query';
import { ColumnDef } from '@tanstack/react-table';
import { KeyRound, MoreHorizontal, Pencil, Trash2, Variable } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { toast } from 'sonner';

type BindingPageAction = 'add-variable' | 'add-secret-key' | 'create-secret';
type EntryDialogMode = 'add-variable' | 'add-secret-key' | 'edit';

const bindingActions = new EventTarget();
const EMPTY_ENTRIES: ResourceBindingView[] = [];
const EMPTY_SECRETS: SecretDefinitionView[] = [];

export const BindingsAddButton = () => {
  const { data: configData } = useRead('getGlobalResourceBindings');
  const { data: secretsData } = useRead('listSecretDefinitions');
  const secrets = useMemo(() => secretsData?.data.secrets ?? EMPTY_SECRETS, [secretsData?.data.secrets]);
  const canWrite = Boolean(configData?.data.capabilities?.canWrite);
  const canCreateSecret = Boolean(secretsData?.data.capabilities.canWrite);

  const dispatch = (action: BindingPageAction) => {
    bindingActions.dispatchEvent(new CustomEvent<BindingPageAction>('bindings-action', { detail: action }));
  };

  return (
    <ResourceBindingAddDropdown
      disabled={!canWrite}
      hasSecrets={secrets.length > 0}
      canCreateSecret={canCreateSecret}
      onAddVariable={() => dispatch('add-variable')}
      onBindSecret={() => dispatch('add-secret-key')}
      onCreateSecret={() => dispatch('create-secret')}
    />
  );
};

export const BindingsGroupActions = ({ items }: { items: ResourceBindingView[] }) => (
  <ActionBar type="Binding" items={items} actions={[DeleteSelectedVariablesAction]} />
);

const DeleteSelectedVariablesAction: ButtonGroupComponent<ResourceBindingView> = ({ resources }) => {
  const selected = Array.isArray(resources) ? resources : [resources];
  const queryClient = useQueryClient();
  const { data } = useRead('getGlobalResourceBindings');
  const replace = useMutate('replaceGlobalResourceBindings');
  const [, setSelectedResources] = useSelectedResources<ResourceBindingView>('Binding');
  const [open, setOpen] = useState(false);
  const allEntries = data?.data.entries ?? EMPTY_ENTRIES;
  const canWrite = Boolean(data?.data.capabilities?.canWrite);

  const deleteSelected = async () => {
    if (!canWrite) return;

    try {
      const ids = new Set(selected.map((entry) => entry.id));
      const next = allEntries.filter((entry) => !ids.has(entry.id)).map(toInputFromView);
      await replace.mutateAsync({ data: { entries: next } } as any);
      await invalidateConfigurationQueries(queryClient);
      setSelectedResources([]);
      setOpen(false);
      toast.success(`${selected.length} ${selected.length === 1 ? 'entry' : 'entries'} deleted`);
    } catch {
      toast.error(replace.validationErrors ?? 'Failed to delete selected entries');
    }
  };

  return (
    <>
      <Button type="button" variant="destructive" size="sm" disabled={!canWrite} onClick={() => setOpen(true)}>
        <Trash2 className="h-3.5 w-3.5" />
        Delete
      </Button>
      <ConfirmDeleteDialog
        type="Binding"
        open={open}
        count={selected.length}
        isPending={replace.isPending}
        onOpenChange={setOpen}
        onConfirm={deleteSelected}
      />
    </>
  );
};

export const BindingsTable = ({
  items,
  isLoading,
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
  const replace = useMutate('replaceGlobalResourceBindings');
  const [, setSelectedResources] = useSelectedResources<ResourceBindingView>('Binding');
  const allEntries = data?.data.entries ?? EMPTY_ENTRIES;
  const canWrite = Boolean(data?.data.capabilities?.canWrite);
  const canCreateSecret = Boolean(secretsData?.data.capabilities.canWrite);
  const originalInputs = useMemo(() => allEntries.map(toInputFromView), [allEntries]);
  const secrets = useMemo(() => secretsData?.data.secrets ?? EMPTY_SECRETS, [secretsData?.data.secrets]);
  const secretNames = useMemo(() => new Map(secrets.map((secret) => [secret.id, secret.name])), [secrets]);
  const [dialogMode, setDialogMode] = useState<EntryDialogMode>('add-variable');
  const [editingEntry, setEditingEntry] = useState<ResourceBindingView | null>(null);
  const [entryInput, setEntryInput] = useState<ResourceBindingInput>(newVariableInput());
  const [entryDialogOpen, setEntryDialogOpen] = useState(false);
  const [deletingEntry, setDeletingEntry] = useState<ResourceBindingView | null>(null);

  const saveEntries = useCallback(
    async (entries: ResourceBindingInput[], successMessage?: string) => {
      await replace.mutateAsync({ data: { entries } } as any);
      await invalidateConfigurationQueries(queryClient);
      if (successMessage) {
        toast.success(successMessage);
      }
    },
    [queryClient, replace],
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

        await saveEntries(next, 'Secret key added');
        return true;
      },
      [originalInputs, saveEntries],
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
    const listener = (event: Event) => {
      if (!canWrite) return;

      const action = (event as CustomEvent<BindingPageAction>).detail;
      if (action === 'add-variable') openEntryDialog('add-variable');
      if (action === 'add-secret-key') openEntryDialog('add-secret-key');
      if (action === 'create-secret') openCreateSecretDialog(true);
    };

    bindingActions.addEventListener('bindings-action', listener);
    return () => bindingActions.removeEventListener('bindings-action', listener);
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

      await saveEntries(next, dialogMode === 'edit' ? 'Entry updated' : 'Entry created');
      setEntryDialogOpen(false);
      setEditingEntry(null);
    } catch {
      toast.error(replace.validationErrors ?? 'Failed to save entry');
    }
  };

  const deleteEntry = async () => {
    if (!deletingEntry) return;
    if (!canWrite) return;

    try {
      const next = allEntries.filter((entry) => entry.id !== deletingEntry.id).map(toInputFromView);
      await saveEntries(next, 'Entry deleted');
      setDeletingEntry(null);
      setSelectedResources([]);
    } catch {
      toast.error(replace.validationErrors ?? 'Failed to delete entry');
    }
  };

  const cols = useMemo(
    () =>
      columns({
        secretNames,
        disabled: !canWrite,
        onEdit: (entry) => openEntryDialog('edit', entry),
        onDelete: setDeletingEntry,
      }),
    [canWrite, openEntryDialog, secretNames],
  );

  return (
    <div className="flex w-full flex-col gap-6">
      <div className="flex flex-col gap-6">
        <div className="flex items-center gap-3">
          <div className="inline-flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10 text-primary">
            <Variable className="h-4 w-4" />
          </div>
          <div>
            <div className="font-bold">Bindings</div>
            <p className="text-xs text-muted-foreground">
              Global variables, secret keys, and providers inherited by stacks and deployments.
            </p>
          </div>
        </div>

        <ContentCard>
          <DataTable
            columns={cols}
            data={items ?? EMPTY_ENTRIES}
            isLoading={isLoading}
            onSelectionChange={setSelectedResources}
          />
        </ContentCard>
      </div>

      <EntryEditorDialog
        open={entryDialogOpen}
        mode={dialogMode}
        input={entryInput}
        secrets={secrets}
        isPending={replace.isPending}
        onOpenChange={setEntryDialogOpen}
        onInputChange={setEntryInput}
        onSave={saveEntry}
      />
      <CreateSecretDialog {...secretCreation.dialogProps} />
      <ConfirmDeleteDialog
        type="Binding"
        open={Boolean(deletingEntry)}
        count={1}
        isPending={replace.isPending}
        onOpenChange={(open) => !open && setDeletingEntry(null)}
        onConfirm={deleteEntry}
      />
    </div>
  );
};

const columns = ({
  secretNames,
  disabled,
  onEdit,
  onDelete,
}: {
  secretNames: Map<string, string>;
  disabled: boolean;
  onEdit: (entry: ResourceBindingView) => void;
  onDelete: (entry: ResourceBindingView) => void;
}): ColumnDef<ResourceBindingView>[] => [
  {
    id: 'select',
    header: ({ table }) => (
      <Checkbox
        checked={table.getIsAllPageRowsSelected() || (table.getIsSomePageRowsSelected() && 'indeterminate')}
        onCheckedChange={(value) => table.toggleAllPageRowsSelected(!!value)}
        aria-label="Select all"
      />
    ),
    cell: ({ row }) => (
      <Checkbox
        checked={row.getIsSelected()}
        onCheckedChange={(value) => row.toggleSelected(!!value)}
        aria-label="Select binding"
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'name',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => <NameCell entry={row.original} />,
    sortingFn: (rowA: any, rowB: any): number => rowA.original.name.localeCompare(rowB.original.name),
  },
  {
    accessorKey: 'kind',
    header: ({ column }) => <SortableCell cellName="Type" column={column} />,
    cell: ({ row }) => <KindBadge kind={row.original.kind} />,
    sortingFn: (rowA: any, rowB: any): number => rowA.original.kind.localeCompare(rowB.original.kind),
  },
  {
    accessorKey: 'value',
    header: ({ column }) => <SortableCell cellName="Value" column={column} />,
    cell: ({ row }) => <ValueCell entry={row.original} secretNames={secretNames} />,
  },
  {
    accessorKey: 'secretDeliveryMode',
    header: ({ column }) => <SortableCell cellName="Delivery" column={column} />,
    cell: ({ row }) =>
      row.original.kind === ResourceBindingKind.Secret ? (
        <span className="text-xs text-muted-foreground">{row.original.secretDeliveryMode ?? ENV_DELIVERY_MODE}</span>
      ) : (
        <span className="text-xs text-muted-foreground">-</span>
      ),
  },
  {
    id: 'actions',
    cell: ({ row }) => <EntryActionsCell entry={row.original} disabled={disabled} onEdit={onEdit} onDelete={onDelete} />,
  },
];

const NameCell = ({ entry }: { entry: ResourceBindingView }) => (
  <div className="flex items-center gap-2 py-2">
    {entry.kind === ResourceBindingKind.Secret ? (
      <KeyRound className="h-3.5 w-3.5 text-muted-foreground" />
    ) : (
      <Variable className="h-3.5 w-3.5 text-muted-foreground" />
    )}
    <span className="font-mono text-xs">{entry.name}</span>
  </div>
);

const KindBadge = ({ kind }: { kind: ResourceBindingKind }) => (
  <Badge variant={kind === ResourceBindingKind.Secret ? 'outline' : 'secondary'} className="gap-1">
    {kind === ResourceBindingKind.Secret ? <KeyRound className="size-3" /> : <Variable className="size-3" />}
    {kind === ResourceBindingKind.Secret ? 'Secret' : 'Binding'}
  </Badge>
);

const ValueCell = ({
  entry,
  secretNames,
}: {
  entry: ResourceBindingView;
  secretNames: Map<string, string>;
}) => {
  if (entry.kind === ResourceBindingKind.Secret) {
    const secretName = entry.secretId ? secretNames.get(entry.secretId) : undefined;
    return (
      <div className="flex flex-col gap-0.5 py-2">
        <span className="font-mono text-xs">{secretName ?? 'Unbound secret'}</span>
        <span className="font-mono text-xs text-muted-foreground">********</span>
      </div>
    );
  }

  return <span className="font-mono text-xs">{entry.value}</span>;
};

const EntryActionsCell = ({
  entry,
  disabled,
  onEdit,
  onDelete,
}: {
  entry: ResourceBindingView;
  disabled: boolean;
  onEdit: (entry: ResourceBindingView) => void;
  onDelete: (entry: ResourceBindingView) => void;
}) => (
  <DropdownMenu>
    <DropdownMenuTrigger asChild>
      <Button variant="ghost" className="h-8 w-8 p-0" disabled={disabled}>
        <span className="sr-only">Open menu</span>
        <MoreHorizontal className="h-4 w-4" />
      </Button>
    </DropdownMenuTrigger>
    <DropdownMenuContent align="end" className="w-38 bg-background py-2">
      <DropdownMenuItem onClick={() => onEdit(entry)}>
        <Pencil className="h-3.5 w-3.5" />
        Edit
      </DropdownMenuItem>
      <DropdownMenuSeparator />
      <DropdownMenuItem className="text-danger hover:text-danger!" onClick={() => onDelete(entry)}>
        <Trash2 className="h-3.5 w-3.5" />
        Delete
      </DropdownMenuItem>
    </DropdownMenuContent>
  </DropdownMenu>
);

const EntryEditorDialog = ({
  open,
  mode,
  input,
  secrets,
  isPending,
  onOpenChange,
  onInputChange,
  onSave,
}: {
  open: boolean;
  mode: EntryDialogMode;
  input: ResourceBindingInput;
  secrets: SecretDefinitionView[];
  isPending: boolean;
  onOpenChange: (open: boolean) => void;
  onInputChange: (input: ResourceBindingInput | ((prev: ResourceBindingInput) => ResourceBindingInput)) => void;
  onSave: () => void;
}) => {
  const isSecret = input.kind === ResourceBindingKind.Secret;
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

const hasDuplicateEntryName = (entries: ResourceBindingInput[]): boolean => {
  const names = new Set<string>();
  for (const entry of entries) {
    const name = entry.name.trim().toLowerCase();
    if (names.has(name)) return true;
    names.add(name);
  }

  return false;
};

const invalidateConfigurationQueries = async (queryClient: ReturnType<typeof useQueryClient>) => {
  await queryClient.invalidateQueries({ queryKey: ['getGlobalResourceBindings'] });
  await queryClient.invalidateQueries({ queryKey: ['getResourceBindings'] });
};
