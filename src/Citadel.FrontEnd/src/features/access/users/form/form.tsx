import {
  PatchUserInput,
  CreateUserInput,
  UserResourceAccessInput,
  ResourceInfo,
  LicenseCapability,
  RoleType,
} from '@/api/generated/api.types';
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
import { IdSearchMultiSelectField, extractIds } from '@/components/custom/common';
import { MultiSelect } from '@/components/ui/multi-select';
import { useTeamsList } from '@/features/access/teams/hooks/useTeamsList';
import { ResourceOverridesField } from '@/features/access/overrides/resource-overrides-field';
import { Constants } from '@/lib/constants';
import { useState, useMemo, useCallback } from 'react';
import { useMutate, useRead, useSaveResource } from '@/lib/hooks';
import { useParams } from 'react-router';
import { useDebounce } from '@/hooks/useDebounce';
import { useQueryClient } from '@tanstack/react-query';
import { useLicenseEntitlements } from '@/features/license/use-license-entitlements';
import { LicenseFeatureIndicator } from '@/components/custom/license-feature-indicator';

type UserInput = CreateUserInput | PatchUserInput;

type UserFormResource = Partial<UserInput> & {
  teams?: ResourceInfo[] | null;
  roles?: ResourceInfo[] | null;
};

const normalizeUserResource = (resource?: UserFormResource): UserInput => {
  if (!resource) {
    return {} as UserInput;
  }

  return {
    ...resource,
    teamIds: extractIds(resource.teamIds ?? resource.teams),
    roleIds: extractIds(resource.roleIds ?? resource.roles),
  } as UserInput;
};

const TeamMultiSelectField = ({ value, onChange }: { value: string[] | null; onChange: (ids: string[]) => void }) => {
  const [teamSearch, setTeamSearch] = useState('');
  const debouncedSearch = useDebounce(teamSearch, 300);
  const { pagedUsers, isLoading } = useTeamsList(debouncedSearch || undefined, 20);
  const options = useMemo(
    () => (pagedUsers?.items ?? []).map((team) => ({ label: team.name, value: team.id })),
    [pagedUsers],
  );

  return (
    <IdSearchMultiSelectField
      value={value}
      onChange={onChange}
      options={options}
      isLoading={isLoading}
      searchValue={teamSearch}
      onSearchValueChange={setTeamSearch}
      loadingPlaceholder="Loading teams..."
      selectPlaceholder="Select teams..."
      searchingEmptyIndicator="Searching teams..."
      emptyIndicator="No teams found."
    />
  );
};

export const UserForm = ({
  mode,
  resource,
  disabled,
}: {
  mode: 'add' | 'edit';
  resource?: UserFormResource;
  disabled?: boolean;
}) => {
  const id = useParams().id;
  const [update, setUpdate] = useState<Partial<UserInput>>({});
  const formKey = `${mode}:${id ?? 'new'}`;
  const [confirmPasswordState, setConfirmPasswordState] = useState({ key: formKey, value: '' });
  const confirmPassword = confirmPasswordState.key === formKey ? confirmPasswordState.value : '';
  const setConfirmPassword = useCallback(
    (value: string) => setConfirmPasswordState({ key: formKey, value }),
    [formKey],
  );
  const queryClient = useQueryClient();
  const { hasCapability } = useLicenseEntitlements();
  const canAssignCustomRoles = hasCapability(LicenseCapability.CustomAccessControl);

  const { mutateAsync: createUser } = useMutate('createUser');
  const { mutateAsync: updateUser } = useMutate('updateUser');

  const { data: rolesData, isLoading: rolesLoading } = useRead('listRoles');
  const roleOptions = useMemo(
    () =>
      (rolesData?.data?.roles ?? []).map((role) => {
        const requiresTeamLicense = role.roleType === RoleType.Custom && !canAssignCustomRoles;
        return {
          label: role.name,
          value: role.id,
          disabled: requiresTeamLicense,
          disabledReason: requiresTeamLicense ? 'Requires a Team license' : undefined,
          trailing: requiresTeamLicense ? <LicenseFeatureIndicator edition="Team" /> : undefined,
        };
      }),
    [canAssignCustomRoles, rolesData],
  );

  const refreshData = useCallback(() => {
    localStorage.removeItem(`user:${id ?? 'new'}`);
    queryClient.invalidateQueries({ queryKey: ['getUser', { id }] });
  }, [id, queryClient]);

  const { save: handleSave, isPending } = useSaveResource<UserInput, any>({
    mode,
    basePath: 'access/users',
    entityName: 'User',
    onCreate: (payload) => createUser({ data: payload as CreateUserInput }),
    onUpdate: (payload) => updateUser({ id: id!, data: payload as PatchUserInput }),
    onRefresh: refreshData,
  });

  const wrappedSave = async (payload: UserInput) => {
    const sanitizedPayload = (() => {
      if (mode !== 'edit') return payload;

      const next = { ...(payload as PatchUserInput) } as Record<string, unknown>;
      if (typeof next.password === 'string' && next.password.trim().length === 0) {
        delete next.password;
      }
      return next as unknown as UserInput;
    })();

    if (mode === 'add') {
      const pwd = (payload as CreateUserInput).password ?? '';
      if (confirmPassword !== pwd) return;
    }

    if (mode === 'edit') {
      const pwd = String((payload as PatchUserInput).password ?? '');
      if (pwd && confirmPassword !== pwd) return;
    }

    const result = await handleSave(sanitizedPayload);
    delete (payload as Partial<CreateUserInput>).password;
    setConfirmPassword('');
    return result;
  };

  const original = useMemo(() => normalizeUserResource(resource), [resource]);

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
                      persistDraft: true,
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
                description:
                  mode === 'edit'
                    ? 'Leave blank to keep the current password. New passwords require 15 to 128 characters.'
                    : 'Use 15 to 128 characters.',
                required: mode === 'add',
                validate: (value) => {
                  const password = String(value ?? '');

                  if (mode === 'add' && !password) return 'Required';
                  if (mode === 'edit' && !password) return null;

                  const length = [...password].length;
                  if (length < 15) return 'Password must be at least 15 characters long';
                  if (length > 128) return 'Password must be no more than 128 characters long';
                  return null;
                },
                render: (value, set) => (
                  <FieldInput
                    type="password"
                    value={value ?? ''}
                    onChange={(v) => set({ password: v })}
                    placeholder={mode === 'edit' ? 'Leave blank to keep current password' : undefined}
                  />
                ),
              }),
              {
                kind: 'field' as const,
                field: {
                  key: 'confirmPassword' as any,
                  label: 'Repeat Password',
                  validate: () => {
                    const pwd = String((update as Partial<UserInput>).password ?? '');

                    if (mode === 'edit' && !pwd) return null;
                    if (confirmPassword.length === 0) return 'Required';

                    return confirmPassword !== pwd ? 'Passwords do not match' : null;
                  },
                  render: (_value: any, _set: FieldChange<UserInput>) => {
                    const pwd = String((update as Partial<UserInput>).password ?? '');
                    const mismatch = pwd.length > 0 && confirmPassword.length > 0 && confirmPassword !== pwd;
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
                  <TeamMultiSelectField value={extractIds(value)} onChange={(ids) => set({ teamIds: ids })} />
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
                      defaultValue={extractIds(value)}
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
    [mode, roleOptions, rolesLoading, confirmPassword, setConfirmPassword, update],
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
      disabled={disabled}
      draftKey={`user:${id ?? 'new'}`}
      draftVersion={1}
    />
  );
};
