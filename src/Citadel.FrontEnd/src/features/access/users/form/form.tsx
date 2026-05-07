import { PatchUserInput, CreateUserInput, UserResourceAccessInput } from '@/api/generated/api.types';
import {
  FormShell,
  defineField,
  defineGroupField,
  defineSection,
  FieldInput,
  FieldSwitch,
  type FieldChange,
  type FieldItemConfig,
} from '@/components/custom/form-builder';
import { MultiSelect } from '@/components/ui/multi-select';
import { useTeamsList } from '@/features/access/teams/hooks/useTeamsList';
import { ResourceOverridesField } from '@/features/access/overrides/resource-overrides-field';
import { Constants } from '@/lib/constants';
import { useState, useMemo } from 'react';
import { useMutate, useRead, useSaveResource } from '@/lib/hooks';
import { useParams } from 'react-router';
import { useDebounce } from '@/hooks/useDebounce';

type UserInput = CreateUserInput | PatchUserInput;

const TeamMultiSelectField = ({ value, onChange }: { value: string[] | null; onChange: (ids: string[]) => void }) => {
  const [teamSearch, setTeamSearch] = useState('');
  const debouncedSearch = useDebounce(teamSearch, 300);
  const { pagedUsers, isLoading } = useTeamsList(debouncedSearch || undefined, 20);
  const options = useMemo(
    () => (pagedUsers?.items ?? []).map((team) => ({ label: team.name, value: team.id })),
    [pagedUsers],
  );

  return (
    <div className="max-w-100">
      <MultiSelect
        options={options}
        defaultValue={(value ?? []).map(String)}
        onValueChange={onChange}
        placeholder={isLoading ? 'Loading teams...' : 'Select teams...'}
        searchable
        searchValue={teamSearch}
        onSearchValueChange={setTeamSearch}
        disableLocalSearchFilter
        emptyIndicator={isLoading ? 'Searching teams...' : 'No teams found.'}
        maxCount={5}
        animation={0}
        resetOnDefaultValueChange={false}
      />
    </div>
  );
};

export const UserForm = ({ mode, resource }: { mode: 'add' | 'edit'; resource?: UserInput }) => {
  const id = useParams().id;
  const [update, setUpdate] = useState<Partial<UserInput>>({});
  const [confirmPassword, setConfirmPassword] = useState('');

  const { mutateAsync: createUser } = useMutate('createUser');
  const { mutateAsync: updateUser } = useMutate('updateUser');

  const { data: rolesData, isLoading: rolesLoading } = useRead('listRoles');
  const roleOptions = useMemo(
    () => (rolesData?.data?.roles ?? []).map((r) => ({ label: r.name, value: r.id })),
    [rolesData],
  );

  const { save: handleSave, isPending } = useSaveResource<UserInput, any>({
    mode,
    basePath: 'users',
    entityName: 'User',
    onCreate: (payload) => createUser({ data: payload as CreateUserInput }),
    onUpdate: (payload) => updateUser({ id: id!, data: payload as PatchUserInput }),
    onRefresh: () => {
      localStorage.removeItem(`User:${id ?? 'new'}`);
    },
  });

  const wrappedSave = async (payload: UserInput) => {
    if (mode === 'add') {
      const pwd = (payload as CreateUserInput).password ?? '';
      if (confirmPassword !== pwd) return;
    }
    return handleSave(payload);
  };

  const original = resource ?? ({} as UserInput);

  const schema = useMemo(
    () => ({
      '': defineSection<UserInput>({
        title: '',
        items: [
          defineGroupField<UserInput>({
            id: 'general',
            label: 'General',
            title: 'General',
            description: 'Basic identity and account state.',
            items: [
              ...(mode === 'add'
                ? [
                    defineField<UserInput, 'name'>({
                      key: 'name',
                      label: 'Username',
                      description: 'Provide a unique name to identify this User.',
                      required: true,
                      validate: (v) =>
                        !new RegExp(Constants.validNameIdentifier).test(v) ? 'Invalid name format' : null,
                      render: (value, set) => (
                        <FieldInput value={value ?? ''} onChange={(v) => set({ name: v })} placeholder="john-doe" />
                      ),
                    }),
                  ]
                : []),
              defineField({
                key: 'email',
                label: 'Email',
                required: true,
                validate: (value) => {
                  const email = String(value ?? '').trim();
                  if (!email) return 'Required';
                  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email) ? null : 'Invalid email address';
                },
                render: (value, set) => (
                  <FieldInput
                    type="email"
                    value={value ?? ''}
                    onChange={(v) => set({ email: v })}
                    placeholder="john@example.com"
                  />
                ),
              }),
              defineField({
                key: 'isEnabled',
                label: 'Enabled',
                description: 'Whether this user account is active.',
                render: (value, set) => (
                  <FieldSwitch checked={value ?? true} id="user-is-enabled" onChange={(v) => set({ isEnabled: v })} />
                ),
              }),
            ],
          }),
          defineGroupField<UserInput>({
            id: 'security',
            label: 'Security',
            items: [
              defineField<UserInput, 'password'>({
                key: 'password',
                label: 'Password',
                required: true,
                validate: (value) => {
                  const password = String(value ?? '');
                  if (!password) return 'Required';
                  return password.length < 6 ? 'Password must be at least 6 characters long' : null;
                },
                render: (value, set) => (
                  <FieldInput type="password" value={value ?? ''} onChange={(v) => set({ password: v })} />
                ),
              }),
              {
                kind: 'field' as const,
                field: {
                  key: 'confirmPassword' as any,
                  label: 'Confirm Password',
                  validate: () => {
                    const pwd = (update as Partial<CreateUserInput>).password ?? '';
                    if (confirmPassword.length === 0) return 'Required';
                    return confirmPassword !== pwd ? 'Passwords do not match' : null;
                  },
                  render: (_value: any, _set: FieldChange<UserInput>) => {
                    const pwd = (update as Partial<CreateUserInput>).password ?? '';
                    const mismatch = confirmPassword.length > 0 && confirmPassword !== pwd;
                    return (
                      <div className="flex flex-col gap-1">
                        <FieldInput
                          type="password"
                          value={confirmPassword}
                          onChange={setConfirmPassword}
                          placeholder="Repeat password"
                        />
                        {mismatch && <p className="text-xs text-destructive">Passwords do not match</p>}
                      </div>
                    );
                  },
                },
              } as FieldItemConfig<UserInput>,
            ],
          }),
          defineGroupField<UserInput>({
            id: 'teams',
            label: 'Teams',
            items: [
              defineField<UserInput, 'teamIds'>({
                key: 'teamIds',
                label: 'Teams',
                description: 'Add this user to teams. Users inherit all roles assigned to their teams.',
                render: (value, set) => (
                  <TeamMultiSelectField value={value ?? []} onChange={(ids) => set({ teamIds: ids })} />
                ),
              }),
            ],
          }),
          defineGroupField<UserInput>({
            id: 'roles',
            label: 'Roles',
            items: [
              defineField<UserInput, 'roleIds'>({
                key: 'roleIds',
                label: 'Roles',
                description: 'Assign roles directly to this user to define what they can access and manage.',
                render: (value, set) => (
                  <div className="max-w-100">
                    <MultiSelect
                      options={roleOptions}
                      defaultValue={(value ?? []).map(String)}
                      onValueChange={(vals) => set({ roleIds: vals })}
                      placeholder={rolesLoading ? 'Loading...' : 'Select roles...'}
                      disabled={rolesLoading}
                      maxCount={5}
                      animation={0}
                      resetOnDefaultValueChange={true}
                    />
                  </div>
                ),
              }),
            ],
          }),
        ],
      }),
      Advanced: defineSection<UserInput>({
        title: 'Advanced',
        items: [
          defineField({
            key: 'resourceAccesses',
            label: 'Overrides',
            description:
              'Grant this user direct access to specific resources outside of their team and role assignments.',
            render: (value, set) => (
              <ResourceOverridesField
                value={(value as UserResourceAccessInput[] | null) ?? []}
                onChange={(next) => set({ resourceAccesses: next })}
              />
            ),
          }),
        ],
      }),
    }),
    [mode, roleOptions, rolesLoading, confirmPassword, update],
  );

  return (
    <FormShell
      mode={mode}
      schema={schema}
      original={original}
      update={update}
      setUpdate={setUpdate}
      onSave={wrappedSave}
      pending={isPending}
      draftKey={`User:${id ?? 'new'}`}
      draftVersion={1}
    />
  );
};
