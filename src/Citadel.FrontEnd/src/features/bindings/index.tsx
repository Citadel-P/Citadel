import {
  ResourceBindingView,
  CreateVaultKvV2SecretProviderInput,
  ResourceCapabilities,
  SecretProviderView,
  TestVaultKvV2SecretProviderConnectionInput,
  UpdateVaultKvV2SecretProviderInput,
} from '@/api/generated/api.types';
import { ActionWithDialog } from '@/components/custom/action-with-dialog';
import { ActionBar } from '@/components/custom/action-bar';
import { IntegrationAddCard, IntegrationCard } from '@/components/custom/common';
import { FieldInput } from '@/components/custom/form-builder';
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
import { CitadelIcons } from '@/lib/icons';
import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { useMutate, useRead } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { KeyRound, LoaderCircle, Plus, Trash2 } from 'lucide-react';
import { useMemo, useState } from 'react';
import { toast } from 'sonner';
import { BindingDropdownActions, BindingGroupActions } from './actions';
import { BindingsAddButton, BindingsTable } from './table';

const EMPTY_CAPABILITIES: ResourceCapabilities = { canRead: false, canWrite: false, canExecute: false };
const PROVIDER_INPUT: CreateVaultKvV2SecretProviderInput = {
  name: '',
  address: '',
  mountPath: 'secret',
  token: '',
};

function SecretProviderCard({
  provider,
  onEdit,
  onDelete,
  disabled = false,
}: {
  provider: SecretProviderView;
  onEdit: () => void;
  onDelete: (id: string) => Promise<void>;
  disabled?: boolean;
}) {
  const letter = provider.name.charAt(0).toUpperCase();

  return (
    <IntegrationCard
      title={provider.name}
      subtitle={provider.address}
      disabled={disabled}
      onEdit={onEdit}
      icon={
        <div className="flex h-10 w-10 items-center justify-center rounded-lg border border-violet-600/10 bg-violet-600 font-bold text-white shadow-sm">
          {letter}
        </div>
      }
      footerLeft={<span className="text-xs font-medium text-zinc-600">Vault KV v2</span>}
      footerRight={
        <div className="flex items-center gap-2">
          <span className="rounded-sm border px-2 py-0.5 text-xs text-muted-foreground">{provider.mountPath}</span>
          <ActionWithDialog
            name={provider.name}
            title="Delete"
            icon={<Trash2 className="h-3.5 w-3.5" />}
            variant="outline"
            disabled={disabled}
            onClick={() => onDelete(provider.id)}
          />
        </div>
      }
    />
  );
}

function SecretProvidersSection() {
  const queryClient = useQueryClient();
  const { data: configurationData } = useRead('getGlobalResourceBindings');
  const { data, isLoading, refetch } = useRead('listSecretProviders');
  const createProvider = useMutate('createVaultKvV2SecretProvider');
  const updateProvider = useMutate('updateVaultKvV2SecretProvider');
  const deleteProvider = useMutate('deleteSecretProvider');
  const testProvider = useMutate('testVaultKvV2SecretProviderConnection');
  const providers = data?.data.providers ?? [];
  const canWrite = Boolean(configurationData?.data.capabilities?.canWrite);
  const [open, setOpen] = useState(false);
  const [editingProvider, setEditingProvider] = useState<SecretProviderView | null>(null);
  const [input, setInput] = useState<CreateVaultKvV2SecretProviderInput>(PROVIDER_INPUT);

  const openAdd = () => {
    setEditingProvider(null);
    setInput(PROVIDER_INPUT);
    setOpen(true);
  };

  const openEdit = (provider: SecretProviderView) => {
    if (!canWrite) return;

    setEditingProvider(provider);
    setInput({
      name: provider.name,
      address: provider.address,
      mountPath: provider.mountPath,
      token: '',
    });
    setOpen(true);
  };

  const save = async () => {
    try {
      if (editingProvider) {
        const updateInput: UpdateVaultKvV2SecretProviderInput = {
          name: input.name,
          address: input.address,
          mountPath: input.mountPath,
          token: input.token.trim() ? input.token : null,
        };
        await updateProvider.mutateAsync({ id: editingProvider.id, data: updateInput } as any);
      } else {
        await createProvider.mutateAsync({ data: input } as any);
      }

      setInput(PROVIDER_INPUT);
      setEditingProvider(null);
      setOpen(false);
      await refetch();
      await queryClient.invalidateQueries({ queryKey: ['listSecretDefinitions'] });
      toast.success(editingProvider ? 'Secret provider updated' : 'Secret provider created');
    } catch {
      toast.error(
        editingProvider
          ? (updateProvider.validationErrors ?? 'Failed to update secret provider')
          : (createProvider.validationErrors ?? 'Failed to create secret provider'),
      );
    }
  };

  const testConnection = async () => {
    const payload: TestVaultKvV2SecretProviderConnectionInput = {
      providerId: editingProvider?.id ?? null,
      name: input.name,
      address: input.address,
      mountPath: input.mountPath,
      token: input.token.trim() ? input.token : null,
    };

    try {
      const result = await testProvider.mutateAsync({ data: payload } as any);
      if (result.data.success) {
        toast.success(result.data.message);
      } else {
        toast.error(result.data.message);
      }
    } catch {
      // toast.error(testProvider.validationErrors ?? 'Failed to test secret provider connection');
    }
  };

  const remove = async (id: string) => {
    try {
      await deleteProvider.mutateAsync({ id } as any);
      await refetch();
      await queryClient.invalidateQueries({ queryKey: ['listSecretDefinitions'] });
      toast.success('Secret provider deleted');
    } catch (error) {
      toast.error(deleteProvider.validationErrors ?? 'Failed to delete secret provider');
      throw error;
    }
  };

  const isSaving = createProvider.isPending || updateProvider.isPending;
  const isBusy = isSaving || testProvider.isPending;
  const isEditing = editingProvider != null;
  const testsStoredToken = isEditing && !input.token.trim();
  const canTest =
    canWrite &&
    !testProvider.isPending &&
    input.address.trim().length > 0 &&
    input.mountPath.trim().length > 0 &&
    (isEditing || input.token.trim().length > 0);

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
        <Button variant="outline" disabled={!canWrite} onClick={openAdd}>
          <Plus className="h-3 w-3" /> Create Vault Provider
        </Button>
      </div>

      <div className="grid grid-cols-1 gap-4 md:grid-cols-3 lg:grid-cols-4">
        {providers.map((provider) => (
          <SecretProviderCard
            key={provider.id}
            provider={provider}
            onEdit={() => openEdit(provider)}
            onDelete={remove}
            disabled={!canWrite}
          />
        ))}

        {!isLoading && <IntegrationAddCard label="Create Vault Provider" disabled={!canWrite} onClick={openAdd} />}
      </div>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="sm:max-w-137.5" onInteractOutside={(event) => event.preventDefault()}>
          <DialogHeader>
            <DialogTitle>{isEditing ? 'Edit Vault Provider' : 'Create Vault Provider'}</DialogTitle>
            <DialogDescription>
              {isEditing
                ? 'Update provider metadata. Leave the token blank to keep the existing stored token.'
                : 'Configure a Vault-compatible KV v2 endpoint used by external secrets.'}
            </DialogDescription>
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
                {isEditing
                  ? 'Leave blank to keep the existing encrypted token. Enter a value only to rotate it.'
                  : 'Stored encrypted and used only when resolving external secrets.'}
              </div>
            </div>
          </div>

          <DialogFooter className="flex w-full items-center justify-between sm:justify-between">
            <Button variant="outline" onClick={testConnection} disabled={!canTest || isSaving}>
              {testsStoredToken ? 'Test Stored Token' : 'Test Connection'}
              {testProvider.isPending && <LoaderCircle className="ml-1 h-3.5 w-3.5 animate-spin" />}
            </Button>
            <div className="flex gap-2">
              <Button variant="outline" onClick={() => setOpen(false)} disabled={isBusy}>
                Cancel
              </Button>
              <Button
                onClick={save}
                disabled={
                  isBusy ||
                  !input.name.trim() ||
                  !input.address.trim() ||
                  !input.mountPath.trim() ||
                  (!isEditing && !input.token.trim())
                }>
                Save {isSaving && <LoaderCircle className="ml-1 h-3.5 w-3.5 animate-spin" />}
              </Button>
            </div>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

export const BindingComponents: RequiredComponents<ResourceBindingView> = {
  Icon: CitadelIcons.Binding,
  Content: ({ items, actions, isLoading, isFiltered }) => (
    <div className="flex flex-col gap-6">
      <BindingsTable items={items} actions={actions} isLoading={isLoading} isFiltered={isFiltered} />
      <div className="border-b border-dashed" />
      <SecretProvidersSection />
    </div>
  ),
  DropdownActions: BindingDropdownActions,
  GroupActions: ({ items }) => <ActionBar type="Binding" items={items} actions={Object.values(BindingGroupActions)} />,
  header: {
    title: 'Bindings',
    subtitle: 'Manage global variables, secret keys, and providers inherited by stacks and deployments.',
    showSearch: true,
    showAdd: false,
    Extra: BindingsAddButton,
  },
  useData(): ResourceDataHookResult<ResourceBindingView> {
    const { data, isLoading } = useRead('getGlobalResourceBindings');
    const capabilities = data?.data.capabilities ?? EMPTY_CAPABILITIES;
    const entries = useMemo(
      () => (data?.data.entries ?? []).map((entry) => ({ ...entry, capabilities })),
      [data?.data.entries, capabilities],
    );
    return { items: entries, isLoading, capabilities };
  },
  filterItems: (items, search) => {
    if (!search.trim()) return items;
    const s = search.toLowerCase();
    return items.filter(
      (entry) =>
        entry.name?.toLowerCase().includes(s) ||
        entry.kind?.toLowerCase().includes(s) ||
        entry.value?.toLowerCase().includes(s),
    );
  },
};
