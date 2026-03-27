import { GitRepositoryInput, GitAccountView, GitTransport } from '@/api/generated/api.types';
import {
  FormShell,
  defineField,
  defineGroupField,
  defineSection,
  FieldInput,
  FieldTextArea,
} from '@/components/custom/form-builder';
import { useState, useMemo, useCallback } from 'react';
import { useMutate, useRead } from '@/lib/hooks';
import { toast } from 'sonner';
import { useParams, useNavigate } from 'react-router';
import { useQueryClient } from '@tanstack/react-query';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';

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
      <SelectTrigger className="w-full max-w-[400px]">
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

export const GitRepoForm = ({ mode, resource }: { mode: 'add' | 'edit'; resource?: GitRepositoryInput }) => {
  const id = useParams().id;
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [update, setUpdate] = useState<Partial<GitRepositoryInput>>({});
  const [isPending, setIsPending] = useState(false);

  const { mutateAsync: createGitRepository } = useMutate('createGitRepository');
  const { mutateAsync: updateGitRepository } = useMutate('updateGitRepository');

  const original = useMemo(() => (resource ?? {}) as GitRepositoryInput, [resource]);

  const refreshData = useCallback(() => {
    localStorage.removeItem(`GitRepo:${id ?? 'new'}`);
    queryClient.invalidateQueries({ queryKey: ['getGitRepository', { id }] });
  }, [id, queryClient]);

  const handleSave = async (payload: GitRepositoryInput) => {
    setIsPending(true);
    try {
      if (mode === 'edit') {
        await updateGitRepository({ id, data: payload });
        refreshData();
      } else {
        await createGitRepository({ data: payload });
        navigate('/git-repos');
      }

      toast.success(`Repository "${payload.name}" saved successfully`);
    } finally {
      setIsPending(false);
    }
  };

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
        ],
      }),
    }),
    [mode],
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
      draftKey={`GitRepo:${id ?? 'new'}`}
      draftVersion={1}
    />
  );
};
