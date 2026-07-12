import { Plus, KeyRound, LoaderCircle, Trash2 } from 'lucide-react';
import { useState } from 'react';
import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { ActionWithDialog } from '@/components/custom/action-with-dialog';
import { GitReposTable } from './table';
import { useMutate, useRead } from '@/lib/hooks';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import {
  GitAuthType,
  GitTransport,
  GitAccountInput,
  GitAccountView,
  GitAuthConfiguration,
  GitRepositoryView,
} from '@/api/generated/api.types';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Separator } from '@radix-ui/react-dropdown-menu';
import { GitRepoDropdownActions, GitRepoGroupActions } from './actions';
import { ActionBar } from '@/components/custom/action-bar';
import { Label } from '@/components/ui/label';
import { toast } from 'sonner';
import { FieldInput, FieldTextArea, ItemSelector } from '@/components/custom/form-builder';
import { useGitReposGroup } from './hooks/useGitReposGroup';
import { CitadelIcons } from '@/lib/icons';
import { IntegrationAddCard, IntegrationCard } from '@/components/custom/common';

const EMPTY_ACCOUNT: GitAccountInput = {
  name: '',
  domain: '',
  transport: GitTransport.Https,
  authType: GitAuthType.Basic,
  configuration: { $type: 'Basic', username: '', password: '' },
};

const getInitialInput = (c?: GitAccountView & { configuration?: GitAuthConfiguration | null }): GitAccountInput => {
  if (!c) return { ...EMPTY_ACCOUNT };
  const authTypeToConfigType: Record<GitAuthType, 'Basic' | 'Token' | 'SshKey'> = {
    [GitAuthType.Basic]: 'Basic',
    [GitAuthType.Token]: 'Token',
    [GitAuthType.SshKey]: 'SshKey',
  };
  return {
    name: c.name,
    domain: c.domain,
    transport: c.transport,
    authType: c.authType,
    configuration: c.configuration ?? ({ $type: authTypeToConfigType[c.authType] } as GitAuthConfiguration),
  };
};

const transportLabel: Record<GitTransport, string> = {
  [GitTransport.Http]: 'HTTP',
  [GitTransport.Https]: 'HTTPS',
  [GitTransport.Ssh]: 'SSH',
};

const authTypeLabel: Record<GitAuthType, string> = {
  [GitAuthType.Basic]: 'Basic Auth',
  [GitAuthType.Token]: 'Token',
  [GitAuthType.SshKey]: 'SSH Key',
};

const getAuthTypesForTransport = (transport: GitTransport): GitAuthType[] => {
  if (transport === GitTransport.Ssh) return [GitAuthType.SshKey];
  return [GitAuthType.Basic, GitAuthType.Token];
};

const getTransportColor = (transport: GitTransport) => {
  switch (transport) {
    case GitTransport.Ssh:
      return 'bg-amber-600 border-amber-600/10';
    case GitTransport.Https:
      return 'bg-blue-600 border-blue-600/10';
    case GitTransport.Http:
      return 'bg-teal-600 border-teal-600/10';
  }
};

function useGitAccounts() {
  const { data, refetch } = useRead('listGitAccounts');
  const accounts = data?.data?.gitAccounts ?? [];
  const capabilities = data?.data.capabilities;

  const { mutateAsync: create, isPending: creating } = useMutate('createGitAccount');
  const { mutateAsync: update, isPending: updating } = useMutate('updateGitAccount');
  const { mutateAsync: deleteAccounts, isPending: deleting } = useMutate('deleteGitAccounts');

  const save = async (editing: GitAccountView | null, input: GitAccountInput) => {
    if (!input.name) throw new Error('Account name is required.');
    if (!input.domain) throw new Error('Domain is required.');

    if (editing) {
      await update({ id: editing.id, data: input });
    } else {
      await create({ data: input });
    }

    await refetch();
  };

  const removeAccount = async (id: string) => {
    await deleteAccounts({ ids: [id] } as any);
    await refetch();
  };

  return {
    accounts,
    capabilities,
    save,
    removeAccount,
    saving: creating || updating,
    deleting,
  };
}

function AccountCard({
  account,
  onEdit,
  onDelete,
  disabled = false,
}: {
  account: GitAccountView;
  onEdit: () => void;
  onDelete: (id: string) => Promise<void>;
  disabled?: boolean;
}) {
  const color = getTransportColor(account.transport);
  const letter = transportLabel[account.transport].charAt(0);

  return (
    <IntegrationCard
      title={account.name}
      subtitle={account.domain}
      disabled={disabled}
      onEdit={onEdit}
      icon={
        <div
          className={cn(
            'h-10 w-10 rounded-lg flex items-center justify-center border shadow-sm text-white font-bold',
            color,
          )}>
          {letter}
        </div>
      }
      footerLeft={
        <div className="flex items-center gap-1.5">
          <span className="text-xs font-medium text-zinc-600">
            {transportLabel[account.transport]} · {authTypeLabel[account.authType]}
          </span>
        </div>
      }
      footerRight={
        <ActionWithDialog
          name={account.name}
          title="Delete"
          icon={<Trash2 className="h-3.5 w-3.5" />}
          variant="outline"
          disabled={disabled}
          onClick={() => onDelete(account.id)}
        />
      }
    />
  );
}
function GitAccountsSection() {
  const { accounts, capabilities, save, saving, removeAccount } = useGitAccounts();

  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState<(GitAccountView & { configuration?: GitAuthConfiguration | null }) | null>(
    null,
  );
  const [input, setInput] = useState<GitAccountInput>(EMPTY_ACCOUNT);

  const openAdd = () => {
    setEditing(null);
    setInput(getInitialInput());
    setOpen(true);
  };

  const openEdit = async (account: GitAccountView) => {
    setEditing(account);
    setInput(getInitialInput(account));
    setOpen(true);
  };

  const handleSave = async () => {
    try {
      await save(editing, input);
      toast.success(`Account ${editing ? 'updated' : 'created'} successfully!`);
      setOpen(false);
    } catch {
      // toast.error(e.message ?? 'Operation failed.');
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await removeAccount(id);
      toast.success('Account deleted successfully!');
    } catch (e: any) {
      toast.error(e.message ?? 'Failed to delete account.');
      throw e;
    }
  };

  const handleTransportChange = (transport: GitTransport) => {
    if (transport === GitTransport.Ssh) {
      setInput({
        ...input,
        transport,
        authType: GitAuthType.SshKey,
        configuration: getConfigForAuthType(GitAuthType.SshKey),
      });
      return;
    }
    const validAuth = getAuthTypesForTransport(transport);
    const authType = validAuth.includes(input.authType) ? input.authType : GitAuthType.Basic;
    const config = getConfigForAuthType(authType);
    setInput({ ...input, transport, authType, configuration: config });
  };

  const handleAuthTypeChange = (authType: GitAuthType) => {
    setInput({ ...input, authType, configuration: getConfigForAuthType(authType) });
  };

  const getConfigForAuthType = (authType: GitAuthType): GitAuthConfiguration => {
    switch (authType) {
      case GitAuthType.Basic:
        return { $type: 'Basic', username: '', password: '' };
      case GitAuthType.Token:
        return { $type: 'Token', token: '' };
      case GitAuthType.SshKey:
      default:
        return { $type: 'SshKey', username: '', privateKey: '', passphrase: null };
    }
  };

  const availableAuthTypes = getAuthTypesForTransport(input.transport);

  return (
    <div className="flex flex-col gap-6">
      <div className="flex justify-between">
        <div className="flex items-center gap-3">
          <div className="inline-flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10 text-primary">
            <KeyRound className="h-4 w-4" />
          </div>
          <div>
            <div className="font-bold">Git Accounts</div>
            <p className="text-xs text-muted-foreground">Authentication credentials for your Git providers.</p>
          </div>
        </div>
        <Button variant="outline" disabled={!capabilities?.canWrite} onClick={openAdd}>
          <Plus className="h-3 w-3" /> Add Account
        </Button>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-4 gap-4">
        {accounts.map((a) => (
          <AccountCard
            key={a.id}
            account={a}
            onEdit={() => openEdit(a)}
            onDelete={handleDelete}
            disabled={!capabilities?.canWrite}
          />
        ))}

        <IntegrationAddCard label="Add New Account" disabled={!capabilities?.canWrite} onClick={openAdd} />
      </div>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="sm:max-w-137.5" onInteractOutside={(e) => e.preventDefault()}>
          <DialogHeader>
            <DialogTitle>{editing ? 'Edit' : 'Add'} Git Account</DialogTitle>
            <DialogDescription>
              {editing
                ? 'Update your Git account credentials.'
                : 'Connect a new Git account for repository authentication.'}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-4">
            <div>
              <Label>Name</Label>
              <FieldInput
                className="max-w-full"
                placeholder="e.g. My GitHub Account"
                value={input.name}
                onChange={(v) => setInput({ ...input, name: v })}
              />
            </div>
            <div>
              <Label>Domain</Label>
              <FieldInput
                className="max-w-full"
                placeholder="e.g. github.com"
                value={input.domain}
                onChange={(v) => setInput({ ...input, domain: v })}
              />
            </div>
            <div>
              <Label>Transport</Label>
              <ItemSelector
                className="max-w-full"
                collection={Object.fromEntries(Object.values(GitTransport).map((t) => [t, transportLabel[t]]))}
                value={input.transport}
                onChange={(v: GitTransport) => handleTransportChange(v)}
              />
            </div>
            {input.transport !== GitTransport.Ssh && (
              <div>
                <Label>Auth Type</Label>
                <ItemSelector
                  className="max-w-full"
                  collection={Object.fromEntries(availableAuthTypes.map((t) => [t, authTypeLabel[t]]))}
                  value={input.authType}
                  onChange={(v: GitAuthType) => handleAuthTypeChange(v)}
                />
              </div>
            )}

            {input.authType === GitAuthType.Basic && (
              <>
                <div>
                  <Label>Username</Label>
                  <FieldInput
                    className="max-w-full"
                    placeholder="username"
                    value={(input.configuration as any)?.username ?? ''}
                    onChange={(v) =>
                      setInput({
                        ...input,
                        configuration: { ...input.configuration, $type: 'Basic', username: v } as any,
                      })
                    }
                  />
                </div>
                <div>
                  <Label>Password</Label>
                  <FieldInput
                    className="max-w-full"
                    type="password"
                    placeholder="password"
                    value={(input.configuration as any)?.password ?? ''}
                    onChange={(v) =>
                      setInput({
                        ...input,
                        configuration: { ...input.configuration, $type: 'Basic', password: v } as any,
                      })
                    }
                  />
                </div>
              </>
            )}

            {input.authType === GitAuthType.Token && (
              <div>
                <Label>Token</Label>
                <FieldInput
                  className="max-w-full"
                  type="password"
                  placeholder="personal access token"
                  value={(input.configuration as any)?.token ?? ''}
                  onChange={(v) =>
                    setInput({
                      ...input,
                      configuration: { $type: 'Token', token: v } as any,
                    })
                  }
                />
              </div>
            )}

            {input.authType === GitAuthType.SshKey && (
              <>
                <div>
                  <Label>Username</Label>
                  <FieldInput
                    className="max-w-full"
                    placeholder="git"
                    value={(input.configuration as any)?.username ?? ''}
                    onChange={(v) =>
                      setInput({
                        ...input,
                        configuration: { ...input.configuration, $type: 'SshKey', username: v } as any,
                      })
                    }
                  />
                </div>
                <div>
                  <Label>Private Key</Label>
                  <FieldTextArea
                    placeholder="-----BEGIN OPENSSH PRIVATE KEY-----"
                    value={(input.configuration as any)?.privateKey ?? ''}
                    onChange={(v) =>
                      setInput({
                        ...input,
                        configuration: { ...input.configuration, $type: 'SshKey', privateKey: v } as any,
                      })
                    }
                  />
                </div>
                <div>
                  <Label>Passphrase (optional)</Label>
                  <FieldInput
                    className="max-w-full"
                    type="password"
                    placeholder="passphrase"
                    value={(input.configuration as any)?.passphrase ?? ''}
                    onChange={(v) =>
                      setInput({
                        ...input,
                        configuration: { ...input.configuration, $type: 'SshKey', passphrase: v || null } as any,
                      })
                    }
                  />
                </div>
              </>
            )}
          </div>

          <DialogFooter className="flex w-full justify-end items-center">
            <div className="flex gap-2">
              <Button variant="outline" onClick={() => setOpen(false)} disabled={saving}>
                Cancel
              </Button>
              <Button onClick={handleSave} disabled={saving}>
                Save {saving && <LoaderCircle className="ml-1 h-3.5 w-3.5 animate-spin" />}
              </Button>
            </div>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

const EMPTY_GIT_REPOS: never[] = [];

export const GitRepoComponents: RequiredComponents = {
  Icon: CitadelIcons.GitRepository,
  Content: ({ items, actions, isLoading }) => (
    <div className="flex flex-col gap-6">
      <GitReposTable items={items} actions={actions} isLoading={isLoading} />
      <Separator className="border-b border-dashed" />
      <GitAccountsSection />
    </div>
  ),
  DropdownActions: GitRepoDropdownActions,
  GroupActions: ({ items }) => (
    <ActionBar type="GitRepository" items={items} actions={Object.values(GitRepoGroupActions)} />
  ),
  header: {
    title: 'Git Repositories',
    subtitle: 'Manage your Git repository and account configurations.',
    showSearch: true,
    showAdd: true,
    showTagFilter: true,
    addButtonTitle: 'Add Repository',
  },
  useData(): ResourceDataHookResult<GitRepositoryView> {
    const { gitRepos, capabilities, isLoading } = useGitReposGroup();
    return { items: gitRepos ?? EMPTY_GIT_REPOS, isLoading, capabilities };
  },
  filterItems: (items, search) => {
    if (!search.trim()) return items;
    const s = search.toLowerCase();
    return items.filter(
      (v) =>
        v.name?.toLowerCase().includes(s) ||
        v.url?.toLowerCase().includes(s) ||
        v.id?.toLowerCase().includes(s) ||
        v.id?.substring(0, 12).toLowerCase().includes(s),
    );
  },
};
