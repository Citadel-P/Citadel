import {
  GitAccountView,
  GitTransport,
  GitRepositoryConfigView,
  CreateGitRepositoryInput,
  PatchGitRepositoryInput,
  GitRepositorySyncMode,
  RepoWebhookConfig,
} from '@/api/generated/api.types';
import {
  FormShell,
  defineField,
  defineGroupField,
  defineSection,
  FieldInput,
  FieldTextArea,
} from '@/components/custom/form-builder';
import { useState, useMemo, useCallback, useEffect } from 'react';
import { useMutate, useRead, useSaveResource } from '@/lib/hooks';
import { useParams } from 'react-router';
import { useQueryClient } from '@tanstack/react-query';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { MonacoToArrayEditor } from '@/lib/monaco';
import { WebhookConfigField } from '@/components/custom/webhook-config-field';
import { ResourceTagSelector } from '@/features/tags/components';

const GitAccountSelector = ({
  value,
  onChange,
  disabled,
}: {
  value?: string | null;
  onChange: (v: string | null) => void;
  disabled?: boolean;
}) => {
  const { data } = useRead('listGitAccounts');
  const accounts: GitAccountView[] = data?.data?.gitAccounts ?? [];

  const transportLabel: Record<GitTransport, string> = {
    [GitTransport.Http]: 'HTTP',
    [GitTransport.Https]: 'HTTPS',
    [GitTransport.Ssh]: 'SSH',
  };

  return (
    <Select
      value={value ?? '__none__'}
      onValueChange={(v) => onChange(v === '__none__' ? null : v)}
      disabled={disabled}>
      <SelectTrigger className="w-full max-w-100">
        <SelectValue placeholder="Select a Git account (optional)">
          {value ? (accounts.find((a) => a.id === value)?.name ?? 'Unknown') : 'No account'}
        </SelectValue>
      </SelectTrigger>
      <SelectContent className="bg-background">
        <SelectItem value="__none__">
          <span className="text-muted-foreground">No account (public access)</span>
        </SelectItem>
        {accounts.map((account) => (
          <SelectItem key={account.id} value={account.id}>
            <div className="flex flex-col">
              <span className="font-medium">{account.name}</span>
              <span className="text-xs text-muted-foreground">
                {account.domain} &middot; {transportLabel[account.transport]} &middot; {account.authType}
              </span>
            </div>
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
};
type GitRepositoryInput = CreateGitRepositoryInput | PatchGitRepositoryInput;

export const GitRepoForm = ({
  mode,
  metadataChanged,
  disabled,
}: {
  mode: 'add' | 'edit';
  metadataChanged?: boolean;
  disabled?: boolean;
}) => {
  const id = useParams().id;
  const queryClient = useQueryClient();
  const [update, setUpdate] = useState<Partial<GitRepositoryInput>>({});

  const { mutateAsync: createGitRepository } = useMutate('createGitRepository');
  const { mutateAsync: updateGitRepository } = useMutate('updateGitRepository');
  const { data: gitRepoCfg } = useRead('getGitRepositoryConfig', { id });

  const resource: GitRepositoryConfigView | undefined = gitRepoCfg?.data;

  const original =
    resource ??
    ({
      syncMode: GitRepositorySyncMode.PullInterval,
      syncIntervalMinutes: 5,
      webhook: { enabled: false },
    } as GitRepositoryConfigView);
  const currentSyncMode =
    update.syncMode ?? original.syncMode ?? GitRepositorySyncMode.PullInterval;

  const refreshData = useCallback(() => {
    localStorage.removeItem(`GitRepo:${id ?? 'new'}`);
    queryClient.invalidateQueries({ queryKey: ['listGitRepositories'] });
    queryClient.invalidateQueries({ queryKey: ['getGitRepositoryConfig', { id }] });
  }, [id, queryClient]);

  useEffect(() => {
    if (!metadataChanged) return;
    refreshData();
  }, [metadataChanged, refreshData]);

  const { save: handleSave, isPending } = useSaveResource<GitRepositoryInput, any>({
    mode,
    basePath: 'git-repos',
    entityName: 'Repository',
    onCreate: (payload) => createGitRepository({ data: payload as CreateGitRepositoryInput }),
    onUpdate: (payload) => updateGitRepository({ id, data: payload }),
    onRefresh: refreshData,
  });

  const schema = useMemo(
    () => ({
      '': defineSection<GitRepositoryInput>({
        title: '',
        items: [
          ...(mode === 'add'
            ? [
                defineGroupField<GitRepositoryInput>({
                  id: 'details',
                  label: 'Details',
                  items: [
                    defineField({
                      key: 'name',
                      label: 'Name',
                      required: true,
                      description: 'A unique name to identify this repository.',
                      validate: (v) => (!v ? 'Name is required' : null),
                      render: (val, set) => (
                        <FieldInput value={val} onChange={(v) => set({ name: v })} placeholder="e.g. my-app-repo" />
                      ),
                    }),
                    defineField({
                      key: 'description',
                      label: 'Description',
                      description: 'Optional notes about this repository.',
                      render: (val, set) => (
                        <FieldTextArea
                          value={val}
                          onChange={(v) => set({ description: v || null })}
                          placeholder="Repository description..."
                        />
                      ),
                    }),
                    defineField({
                      key: 'tagIds',
                      label: 'Tags',
                      description: 'Optional tags for filtering and grouping this repository.',
                      render: (val, set) => (
                        <ResourceTagSelector
                          value={val}
                          disabled={disabled}
                          onChange={(tagIds) => set({ tagIds })}
                        />
                      ),
                    }),
                  ],
                }),
              ]
            : []),

          defineGroupField<GitRepositoryInput>({
            id: 'repository',
            label: 'Repository',
            items: [
              defineField({
                key: 'url',
                label: 'Repo url',
                required: true,
                description: 'The repository URL.',
                validate: (v) => (!v ? 'Repo url is required' : null),
                render: (val, set) => (
                  <FieldInput
                    value={val}
                    onChange={(v) => set({ url: v })}
                    placeholder="e.g. https://github.com/org/repo"
                  />
                ),
              }),
              defineField({
                key: 'defaultBranch',
                label: 'Default Branch',
                required: true,
                description: 'The default branch to use when pulling changes.',
                validate: (v) => (!v ? 'Default branch is required' : null),
                render: (val, set) => (
                  <FieldInput value={val} onChange={(v) => set({ defaultBranch: v })} placeholder="e.g. main" />
                ),
              }),
            ],
          }),

          defineGroupField<GitRepositoryInput>({
            id: 'authentication',
            label: 'Authentication',
            items: [
              defineField({
                key: 'gitAccountId',
                label: 'Git Account',
                description: 'Optionally link a Git account for authentication.',
                render: (val, set) => <GitAccountSelector value={val} onChange={(v) => set({ gitAccountId: v })} />,
              }),
            ],
          }),

          defineGroupField<GitRepositoryInput>({
            id: 'sync',
            label: 'Sync',
            description: 'Choose how Citadel refreshes the local repository cache.',
            items: [
              defineField({
                key: 'syncMode',
                label: 'Sync mode',
                render: (val, set) => (
                  <Select
                    value={val ?? GitRepositorySyncMode.PullInterval}
                    onValueChange={(v) => {
                      const syncMode = v as GitRepositorySyncMode;
                      set({
                        syncMode,
                        syncIntervalMinutes:
                          syncMode === GitRepositorySyncMode.PullInterval
                            ? (update.syncIntervalMinutes ?? original.syncIntervalMinutes ?? 5)
                            : null,
                      });
                    }}>
                    <SelectTrigger className="w-full max-w-100">
                      <SelectValue placeholder="Select sync mode" />
                    </SelectTrigger>
                    <SelectContent className="bg-background">
                      <SelectItem value={GitRepositorySyncMode.PullInterval}>Pull on interval</SelectItem>
                      <SelectItem value={GitRepositorySyncMode.Manual}>Manual only</SelectItem>
                    </SelectContent>
                  </Select>
                ),
              }),
              ...(currentSyncMode === GitRepositorySyncMode.PullInterval
                ? [
                    defineField<GitRepositoryInput, 'syncIntervalMinutes'>({
                      key: 'syncIntervalMinutes',
                      label: 'Pull interval',
                      required: true,
                      description: 'How often Citadel checks this repository for changes, in minutes.',
                      validate: (v) => (v === undefined || v === null || v < 1 ? 'Interval must be at least 1 minute' : null),
                      render: (val, set) => (
                        <FieldInput
                          type="number"
                          value={val ?? 5}
                          onChange={(v) => set({ syncIntervalMinutes: v ?? 5 })}
                          placeholder="5"
                        />
                      ),
                    }),
                  ]
                : []),
            ],
          }),

          defineGroupField<GitRepositoryInput>({
            id: 'on_pull',
            label: 'On Pull',
            title: 'On Pull',
            description: 'Execute a shell command after pulling the repo. The given Cwd is relative to repo root.',
            items: [
              defineField({
                key: 'onPull.path',
                label: 'Path',
                render: (val, set) => (
                  <FieldInput
                    value={val}
                    onChange={(v) =>
                      set((prev) => ({
                        onPull: {
                          ...prev.onPull!,
                          path: v,
                        },
                      }))
                    }
                    placeholder="Command working directory"
                  />
                ),
              }),
              defineField({
                key: 'onPull.commands',
                label: 'Commands',
                required: false,
                render: (value, set) => (
                  <MonacoToArrayEditor
                    value={value}
                    helperText="# Add multiple commands on new lines"
                    language="string_list"
                    onChange={(v: string[] | undefined) =>
                      set((prev) => ({
                        onPull: {
                          ...prev.onPull!,
                          commands: v ?? [],
                        },
                      }))
                    }
                  />
                ),
              }),
            ],
          }),

          defineGroupField<GitRepositoryInput>({
            id: 'on_clone',
            label: 'On Clone',
            title: 'On Clone',
            description: 'Execute a shell command after cloning the repo. The given Cwd is relative to repo root.',
            items: [
              defineField({
                key: 'onClone.path',
                label: 'Path',
                render: (val, set) => (
                  <FieldInput
                    value={val}
                    onChange={(v) =>
                      set((prev) => ({
                        onClone: {
                          ...prev.onClone!,
                          path: v,
                        },
                      }))
                    }
                    placeholder="Command working directory"
                  />
                ),
              }),
              defineField({
                key: 'onClone.commands',
                label: 'Commands',
                required: false,
                render: (value, set) => (
                  <MonacoToArrayEditor
                    value={value}
                    helperText="# Add multiple commands on new lines"
                    language="string_list"
                    onChange={(v: string[] | undefined) =>
                      set((prev) => ({
                        onClone: {
                          ...prev.onClone!,
                          commands: v ?? [],
                        },
                      }))
                    }
                  />
                ),
              }),
            ],
          }),
        ],
      }),
      Advanced: defineSection<GitRepositoryInput>({
        title: 'Advanced',
        items: [
          defineGroupField<GitRepositoryInput>({
            id: 'webhook',
            label: 'Webhook',
            title: 'Webhook',
            description: 'Trigger a repository pull from your Git provider.',
            items: [
              defineField<GitRepositoryInput, 'webhook'>({
                key: 'webhook',
                label: 'Enabled',
                render: (value, set) => (
                  <WebhookConfigField
                    resourceType="repo"
                    resourceId={id}
                    execution="pull"
                    value={(value as RepoWebhookConfig | null) ?? null}
                    defaultBranch={original.defaultBranch}
                    showBranchFilter={false}
                    disabled={disabled}
                    onChange={(webhook) =>
                      set({
                        webhook,
                      })
                    }
                  />
                ),
              }),
            ],
          }),
        ],
      }),
    }),
    [
      mode,
      currentSyncMode,
      original.syncIntervalMinutes,
      original.defaultBranch,
      update.syncIntervalMinutes,
      id,
      disabled,
    ],
  );

  return (
    <FormShell
      mode={mode}
      schema={schema}
      original={original}
      update={update}
      setUpdate={setUpdate}
      onSave={handleSave}
      pending={isPending}
      disabled={disabled}
      draftKey={`GitRepo:${id ?? 'new'}`}
      draftVersion={1}
    />
  );
};
