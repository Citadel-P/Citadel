import { UserResourceAccessInput, ResourceInfo, CreateTeamInput, PatchTeamInput } from '@/api/generated/api.types';
import {
  FormShell,
  defineField,
  defineGroupField,
  defineSection,
  FieldInput,
  FieldSwitch,
} from '@/components/custom/form-builder';
import { IdSearchMultiSelectField, extractIds } from '@/components/custom/common';
import { MultiSelect } from '@/components/ui/multi-select';
import { ResourceOverridesField } from '@/features/access/overrides/resource-overrides-field';
import { Constants } from '@/lib/constants';
import { useState, useMemo, useCallback } from 'react';
import { useMutate, useRead, useSaveResource } from '@/lib/hooks';
import { useParams } from 'react-router';
import { useDebounce } from '@/hooks/useDebounce';
import { useQueryClient } from '@tanstack/react-query';
import { useUsersList } from '../../users/hooks/useUsersList';

type TeamInput = CreateTeamInput | PatchTeamInput;

type TeamFormResource = Partial<TeamInput> & {
  users?: ResourceInfo[] | null;
  roles?: ResourceInfo[] | null;
};

const normalizeTeamResource = (resource?: TeamFormResource): TeamInput => {
  if (!resource) {
    return {} as TeamInput;
  }

  return {
    ...resource,
    userIds: extractIds(resource.userIds ?? resource.users),
    roleIds: extractIds(resource.roleIds ?? resource.roles),
  } as TeamInput;
};

const UserMultiSelectField = ({ value, onChange }: { value: string[] | null; onChange: (ids: string[]) => void }) => {
  const [userSearch, setUserSearch] = useState('');
  const debouncedSearch = useDebounce(userSearch, 300);
  const { pagedUsers, isLoading } = useUsersList(debouncedSearch || undefined, 20);
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
      searchValue={userSearch}
      onSearchValueChange={setUserSearch}
      loadingPlaceholder="Loading users..."
      selectPlaceholder="Select users..."
      searchingEmptyIndicator="Searching users..."
      emptyIndicator="No users found."
    />
  );
};

export const TeamForm = ({
  mode,
  resource,
  disabled,
}: {
  mode: 'add' | 'edit';
  resource?: TeamFormResource;
  disabled?: boolean;
}) => {
  const id = useParams().id;
  const [update, setUpdate] = useState<Partial<TeamInput>>({});
  const queryClient = useQueryClient();

  const { mutateAsync: createTeam } = useMutate('createTeam');
  const { mutateAsync: updateTeam } = useMutate('updateTeam');

  const { data: rolesData, isLoading: rolesLoading } = useRead('listRoles');
  const roleOptions = useMemo(
    () => (rolesData?.data?.roles ?? []).map((r) => ({ label: r.name, value: r.id })),
    [rolesData],
  );

  const refreshData = useCallback(() => {
    localStorage.removeItem(`team:${id ?? 'new'}`);
    queryClient.invalidateQueries({ queryKey: ['getTeam', { id }] });
  }, [id, queryClient]);

  const { save: handleSave, isPending } = useSaveResource<TeamInput, any>({
    mode,
    basePath: 'access/teams',
    entityName: 'Team',
    onCreate: (payload) => createTeam({ data: payload as CreateTeamInput }),
    onUpdate: (payload) => updateTeam({ id: id!, data: payload as PatchTeamInput }),
    onRefresh: refreshData,
  });

  const wrappedSave = async (payload: TeamInput) => {
    const sanitizedPayload = (() => {
      if (mode !== 'edit') return payload;

      const next = { ...(payload as PatchTeamInput) } as Record<string, unknown>;
      if (typeof next.password === 'string' && next.password.trim().length === 0) {
        delete next.password;
      }
      return next as unknown as TeamInput;
    })();

    return handleSave(sanitizedPayload);
  };

  const original = useMemo(() => normalizeTeamResource(resource), [resource]);

  const schema = useMemo(
    () => ({
      '': defineSection<TeamInput>({
        title: '',
        items: [
          defineGroupField<TeamInput>({
            id: 'general',
            label: 'General',
            title: 'General',
            description: 'Core team identity and activation state.',
            items: [
              ...(mode === 'add'
                ? [
                    defineField<TeamInput, 'name'>({
                      key: 'name',
                      label: 'Team Name',
                      description: 'Provide a unique name to identify this team.',
                      required: true,
                      validate: (v) =>
                        !new RegExp(Constants.validNameIdentifier).test(v) ? 'Invalid name format' : null,
                      render: (value, set) => (
                        <FieldInput
                          value={value ?? ''}
                          onChange={(v) => set({ name: v })}
                          placeholder="platform-operators"
                        />
                      ),
                    }),
                  ]
                : []),

              defineField({
                key: 'isEnabled',
                label: 'Enabled',
                description: 'Whether this team is active and can be assigned permissions.',
                render: (value, set) => (
                  <FieldSwitch checked={value ?? true} id="team-is-enabled" onChange={(v) => set({ isEnabled: v })} />
                ),
              }),
            ],
          }),
          defineGroupField<TeamInput>({
            id: 'roles',
            label: 'Roles',
            items: [
              defineField<TeamInput, 'roleIds'>({
                key: 'roleIds',
                label: 'Roles',
                description: 'Assign roles to this team to define what it can access and manage.',
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
          defineGroupField<TeamInput>({
            id: 'users',
            label: 'Users',
            items: [
              defineField<TeamInput, 'userIds'>({
                key: 'userIds',
                label: 'Users',
                description: 'Add users to this team. Members inherit roles and overrides assigned to this team.',
                render: (value, set) => (
                  <UserMultiSelectField value={extractIds(value)} onChange={(ids) => set({ userIds: ids })} />
                ),
              }),
            ],
          }),
        ],
      }),
      Advanced: defineSection<TeamInput>({
        title: 'Advanced',
        items: [
          defineField({
            key: 'resourceAccesses',
            label: 'Overrides',
            description: 'Grant this team direct access to specific resources outside of its role assignments.',
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
    [mode, roleOptions, rolesLoading],
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
      draftKey={`team:${id ?? 'new'}`}
      draftVersion={1}
    />
  );
};
