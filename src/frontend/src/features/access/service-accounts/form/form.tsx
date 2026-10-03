import {
  CreateServiceAccountRequest,
  LicenseCapability,
  ResourceInfo,
  ServiceAccountResourceAccess,
  ResourceType,
  RoleType,
  AddServiceAccountResourceAccessRequest,
  ServiceAccountTokenView,
  ServiceAccountDetailResponse,
  UpdateServiceAccountRequest,
} from '@/api/generated/api.types';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  FieldInput,
  FieldSwitch,
  FormShell,
  defineField,
  defineGroupField,
  defineSection,
} from '@/components/custom/form-builder';
import { IdSearchMultiSelectField, PagedDataTable, extractIds } from '@/components/custom/common';
import { CopyToClipboard } from '@/components/custom/copy-to-clipboard';
import { GenericActionBar } from '@/components/custom/action-bar';
import { ActionButton, ActionWithDialog } from '@/components/custom/action-with-dialog';
import { StateBadge } from '@/components/custom/state-badge';
import { MultiSelect } from '@/components/ui/multi-select';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { ResourceOverridesField } from '@/features/access/overrides/resource-overrides-field';
import { useLicenseEntitlements } from '@/features/license/use-license-entitlements';
import { useTeamsList } from '@/features/access/teams/hooks/useTeamsList';
import { useDebounce } from '@/hooks/useDebounce';
import { Constants } from '@/lib/constants';
import { useMutate, useRead, useSaveResource } from '@/lib/hooks';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { useQueryClient } from '@tanstack/react-query';
import { ColumnDef } from '@tanstack/react-table';
import { AlertTriangle, KeyRound, Trash2 } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Link, useParams } from 'react-router';
import { getTokenExpirationPresets, isValidTokenExpiration } from './token-expiration';

type ServiceAccountInput = CreateServiceAccountRequest;
type ServiceAccountFormResource = Partial<ServiceAccountInput> &
  Pick<ServiceAccountDetailResponse, 'id' | 'name' | 'description' | 'isEnabled' | 'actorId'> & {
    teams?: ResourceInfo[] | null;
    roles?: ResourceInfo[] | null;
    archivedAtUtc?: string | null;
    capabilities?: ServiceAccountDetailResponse['capabilities'];
  };

const normalize = (resource?: ServiceAccountFormResource): ServiceAccountInput =>
  resource
    ? ({
        name: resource.name,
        description: resource.description ?? null,
        isEnabled: resource.isEnabled,
        teamIds: extractIds(resource.teams),
        roleIds: extractIds(resource.roles),
        resourceAccesses: (resource as ServiceAccountDetailResponse).resourceAccesses as AddServiceAccountResourceAccessRequest[],
      } as ServiceAccountInput)
    : { name: '', isEnabled: true, description: null, teamIds: [], roleIds: [], resourceAccesses: [] };

const TeamField = ({ value, onChange }: { value: string[]; onChange: (value: string[]) => void }) => {
  const [search, setSearch] = useState('');
  const debounced = useDebounce(search, 300);
  const { pagedUsers, isLoading } = useTeamsList(debounced || undefined, 20);
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
      searchValue={search}
      onSearchValueChange={setSearch}
      selectPlaceholder="Select teams..."
      loadingPlaceholder="Loading teams..."
      searchingEmptyIndicator="Searching teams..."
      emptyIndicator="No teams found."
    />
  );
};

export const ServiceAccountForm = ({
  mode,
  resource,
  disabled,
}: {
  mode: 'add' | 'edit';
  resource?: ServiceAccountFormResource;
  disabled?: boolean;
}) => {
  const id = useParams().id;
  const queryClient = useQueryClient();
  const { hasCapability } = useLicenseEntitlements();
  const serviceAccountsEnabled = hasCapability(LicenseCapability.CustomAccessControl);
  const [update, setUpdate] = useState<Partial<ServiceAccountInput>>({});
  const [saveConfirmation, setSaveConfirmation] = useState<'admin' | 'disable' | null>(null);
  const saveConfirmationResolver = useRef<((confirmed: boolean) => void) | null>(null);
  const { mutateAsync: create } = useMutate('createServiceAccount');
  const { mutateAsync: patchAccount } = useMutate('updateServiceAccount');
  const { mutateAsync: renameAccount } = useMutate('renameServiceAccount');
  const { mutateAsync: addRole } = useMutate('addServiceAccountRole');
  const { mutateAsync: removeRole } = useMutate('removeServiceAccountRole');
  const { mutateAsync: addTeamMember } = useMutate('addTeamMember');
  const { mutateAsync: removeTeamMember } = useMutate('removeTeamMember');
  const { mutateAsync: addResourceAccess } = useMutate('addServiceAccountResourceAccess');
  const { mutateAsync: removeResourceAccess } = useMutate('removeServiceAccountResourceAccess');
  const usages = useRead('listServiceAccountUsages', { id: id! }, { enabled: mode === 'edit' && Boolean(id) });
  const activeUsageCount = (usages.data?.data ?? []).filter((usage) => usage.isActive).length;
  const { data: rolesData, isLoading: rolesLoading } = useRead('listRoles');
  const roleOptions = useMemo(
    () => (rolesData?.data.roles ?? []).map((role) => ({ label: role.name, value: role.id })),
    [rolesData],
  );
  const adminRoleId = useMemo(
    () =>
      rolesData?.data.roles.find((role) => role.roleType === RoleType.System && role.name.toLowerCase() === 'admin')
        ?.id,
    [rolesData],
  );
  const original = useMemo(() => normalize(resource), [resource]);
  const selectedRoleIds = extractIds((update.roleIds ?? original.roleIds) as string[]);
  const hasAdmin = !!adminRoleId && selectedRoleIds.includes(adminRoleId);
  const originallyHadAdmin = !!adminRoleId && extractIds(original.roleIds as string[]).includes(adminRoleId);

  const resolveAdminConfirmation = useCallback((confirmed: boolean) => {
    const resolve = saveConfirmationResolver.current;
    saveConfirmationResolver.current = null;
    setSaveConfirmation(null);
    resolve?.(confirmed);
  }, []);
  const confirmSave = useCallback(
    (payload: ServiceAccountInput) => {
      const assignsAdmin = !!adminRoleId && extractIds(payload.roleIds as string[]).includes(adminRoleId);
      const disablesAccount = mode === 'edit' && resource?.isEnabled === true && payload.isEnabled === false;
      const confirmation = assignsAdmin && !originallyHadAdmin ? 'admin' : disablesAccount ? 'disable' : null;
      if (!confirmation) return Promise.resolve(true);
      return new Promise<boolean>((resolve) => {
        saveConfirmationResolver.current = resolve;
        setSaveConfirmation(confirmation);
      });
    },
    [adminRoleId, mode, originallyHadAdmin, resource?.isEnabled],
  );

  const refresh = useCallback(() => {
    queryClient.invalidateQueries({ queryKey: ['getServiceAccount', { id }] });
    queryClient.invalidateQueries({ queryKey: ['listServiceAccounts'] });
  }, [id, queryClient]);
  const updateAccount = useCallback(
    async (payload: ServiceAccountInput) => {
      if (!id || !resource) return;
      try {
        if (payload.name !== resource.name) await renameAccount({ data: { id, name: payload.name } });

        const patch: Partial<UpdateServiceAccountRequest> = {};
        if ((payload.description ?? null) !== (resource.description ?? null))
          patch.description = payload.description ?? null;
        if (payload.isEnabled !== resource.isEnabled) patch.isEnabled = payload.isEnabled;
        if (Object.keys(patch).length > 0) await patchAccount({ id, data: patch as UpdateServiceAccountRequest });

        const currentTeamIds = extractIds(resource.teams);
        const nextTeamIds = extractIds(payload.teamIds as string[]);
        for (const teamId of currentTeamIds.filter((teamId) => !nextTeamIds.includes(teamId)))
          await removeTeamMember({ id: teamId, memberActorId: resource.actorId });
        for (const teamId of nextTeamIds.filter((teamId) => !currentTeamIds.includes(teamId)))
          await addTeamMember({ id: teamId, data: { memberActorId: resource.actorId } });

        const currentRoleIds = extractIds(resource.roles);
        const nextRoleIds = extractIds(payload.roleIds as string[]);
        for (const roleId of currentRoleIds.filter((roleId) => !nextRoleIds.includes(roleId)))
          await removeRole({ id, roleId });
        for (const roleId of nextRoleIds.filter((roleId) => !currentRoleIds.includes(roleId)))
          await addRole({ id, data: { roleId } });

        const currentAccesses = (resource.resourceAccesses ?? []) as ServiceAccountResourceAccess[];
        const nextAccesses = (payload.resourceAccesses ?? []) as AddServiceAccountResourceAccessRequest[];
        const key = (access: AddServiceAccountResourceAccessRequest | ServiceAccountResourceAccess) =>
          JSON.stringify([
            access.resourceType,
            access.resourceId,
            access.permissionLevel,
            [...(access.specificPermissions ?? [])].sort(),
          ]);
        const nextKeys = new Set(nextAccesses.map(key));
        const currentKeys = new Set(currentAccesses.map(key));
        for (const access of currentAccesses.filter((access) => !nextKeys.has(key(access)))) {
          if (access.id) await removeResourceAccess({ id, resourceAccessId: access.id });
        }
        for (const access of nextAccesses.filter((access) => !currentKeys.has(key(access))))
          await addResourceAccess({ id, data: access });
      } catch (error) {
        refresh();
        throw error;
      }
    },
    [
      addResourceAccess,
      addRole,
      addTeamMember,
      id,
      patchAccount,
      refresh,
      removeResourceAccess,
      removeRole,
      removeTeamMember,
      renameAccount,
      resource,
    ],
  );

  const { save, isPending } = useSaveResource<ServiceAccountInput, any>({
    mode,
    basePath: 'access/service-accounts',
    entityName: 'Service Account',
    onCreate: (payload) => create({ data: payload as CreateServiceAccountRequest }),
    onUpdate: updateAccount,
    onRefresh: refresh,
  });

  const schema = useMemo(
    () => ({
      '': defineSection<ServiceAccountInput>({
        title: '',
        items: [
          defineGroupField<ServiceAccountInput>({
            id: 'details',
            label: 'Details',
            title: 'Details',
            description: 'Identity and account state for this non-human principal.',
            items: [
              defineField<ServiceAccountInput, 'name'>({
                key: 'name',
                label: 'Name',
                required: true,
                validate: (value) =>
                  !new RegExp(Constants.validNameIdentifier).test(value) ? 'Invalid name format' : null,
                render: (value, set) => (
                  <FieldInput value={value ?? ''} onChange={(name) => set({ name })} placeholder="ci-release" />
                ),
              }),
              defineField<ServiceAccountInput, 'description'>({
                key: 'description',
                label: 'Description',
                render: (value, set) => (
                  <Textarea
                    value={value ?? ''}
                    onChange={(event) => set({ description: event.target.value || null })}
                    placeholder="What this integration is allowed to do"
                    className="max-w-160"
                  />
                ),
              }),
              defineField<ServiceAccountInput, 'isEnabled'>({
                key: 'isEnabled',
                label: 'Enabled',
                description: 'Disabling suspends all credentials without revoking them.',
                render: (value, set) => (
                  <FieldSwitch
                    id="service-account-enabled"
                    checked={value ?? true}
                    disabled={disabled || (!serviceAccountsEnabled && value === false)}
                    onChange={(isEnabled) => set({ isEnabled })}
                  />
                ),
              }),
            ],
          }),
          defineGroupField<ServiceAccountInput>({
            id: 'access',
            label: 'Access',
            description: 'Assign only the permissions required by the integration.',
            items: [
              defineField<ServiceAccountInput, 'teamIds'>({
                key: 'teamIds',
                label: 'Teams',
                render: (value, set) => (
                  <TeamField value={extractIds(value)} onChange={(teamIds) => set({ teamIds })} />
                ),
              }),
              defineField<ServiceAccountInput, 'roleIds'>({
                key: 'roleIds',
                label: 'Roles',
                render: (value, set) => (
                  <div className="max-w-100 space-y-2">
                    <MultiSelect
                      options={roleOptions}
                      defaultValue={extractIds(value)}
                      onValueChange={(roleIds) => set({ roleIds })}
                      placeholder={rolesLoading ? 'Loading...' : 'Select roles...'}
                      disabled={rolesLoading}
                      resetOnDefaultValueChange
                      maxCount={5}
                      animation={0}
                    />
                    {hasAdmin && (
                      <Alert variant="destructive">
                        <AlertTriangle />
                        <AlertTitle>Full administrative access</AlertTitle>
                        <AlertDescription>
                          Every active and future token can fully control Citadel. Prefer a least-privilege custom role.
                        </AlertDescription>
                      </Alert>
                    )}
                  </div>
                ),
              }),
            ],
          }),
        ],
      }),
      Advanced: defineSection<ServiceAccountInput>({
        title: 'Advanced',
        items: [
          defineField<ServiceAccountInput, 'resourceAccesses'>({
            key: 'resourceAccesses',
            label: 'Resource overrides',
            render: (value, set) => (
              <ResourceOverridesField
                value={(value as AddServiceAccountResourceAccessRequest[]) ?? []}
                onChange={(resourceAccesses) =>
                  set({
                    resourceAccesses: resourceAccesses.map((access) => ({
                      ...access,
                      specificPermissions: access.specificPermissions ?? [],
                    })),
                  })
                }
              />
            ),
          }),
        ],
      }),
    }),
    [disabled, hasAdmin, roleOptions, rolesLoading, serviceAccountsEnabled],
  );

  return (
    <div className="space-y-8">
      {hasAdmin && (
        <Alert variant="destructive">
          <AlertTriangle />
          <AlertTitle>Full administrative access</AlertTitle>
          <AlertDescription>
            This Service Account has the Admin role. Every usable token has full Citadel authority.
          </AlertDescription>
        </Alert>
      )}
      {!serviceAccountsEnabled && (
        <Alert>
          <AlertTriangle />
          <AlertTitle>Paused by license</AlertTitle>
          <AlertDescription>
            Existing configuration is preserved. You can disable or archive this account and revoke credentials, but
            creating accounts, enabling them, issuing tokens, or expanding access requires a Team license.
          </AlertDescription>
        </Alert>
      )}
      <FormShell
        mode={mode}
        schema={schema}
        original={original}
        update={update}
        setUpdate={setUpdate}
        onSave={save}
        pending={isPending}
        disabled={disabled || (mode === 'add' && !serviceAccountsEnabled)}
        draftKey={`service-account:${id ?? 'new'}`}
        draftVersion={1}
        confirmSave={confirmSave}
      />
      {mode === 'edit' && id && <UsedByPanel serviceAccountId={id} />}
      <Dialog open={saveConfirmation !== null} onOpenChange={(open) => !open && resolveAdminConfirmation(false)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {saveConfirmation === 'admin' ? 'Grant full administrative access?' : 'Disable Service Account?'}
            </DialogTitle>
            <DialogDescription>
              {saveConfirmation === 'admin'
                ? 'Every active and future token for this Service Account will receive full Citadel authority. Prefer a purpose-specific role with only the permissions the integration needs.'
                : `All credentials will be suspended immediately, but they will not be deleted. Re-enabling the account makes unexpired, unrevoked credentials usable again.${activeUsageCount > 0 ? ` ${activeUsageCount} active scheduled resource${activeUsageCount === 1 ? '' : 's'} use this account; future runs will fail until it is re-enabled or replaced.` : ''}`}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => resolveAdminConfirmation(false)}>
              Cancel
            </Button>
            <Button variant="destructive" onClick={() => resolveAdminConfirmation(true)}>
              {saveConfirmation === 'admin' ? 'Grant Admin' : 'Disable'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
};

export const ServiceAccountTokens = ({ resource }: { resource: ServiceAccountDetailResponse }) => {
  const { hasCapability } = useLicenseEntitlements();
  const serviceAccountsEnabled = hasCapability(LicenseCapability.CustomAccessControl);
  const hasAdmin = resource.roles?.some((role) => role.name.toLowerCase() === 'admin') ?? false;
  const disabled = !!resource.archivedAtUtc || resource.capabilities?.canManageCredentials === false;
  const canCreate = serviceAccountsEnabled && resource.isEnabled;
  const createDisabledReason = !serviceAccountsEnabled
    ? 'Requires a Team license'
    : !resource.isEnabled
      ? 'Enable this Service Account before creating a token'
      : resource.archivedAtUtc
        ? 'Archived Service Accounts cannot create tokens'
        : resource.capabilities?.canManageCredentials === false
          ? 'You do not have permission to manage credentials for this Service Account'
          : undefined;

  return (
    <CredentialsPanel
      serviceAccountId={resource.id}
      hasAdmin={hasAdmin}
      disabled={disabled}
      canCreate={canCreate}
      createDisabledReason={createDisabledReason}
    />
  );
};

const CredentialsPanel = ({
  serviceAccountId,
  hasAdmin,
  disabled,
  canCreate,
  createDisabledReason,
}: {
  serviceAccountId: string;
  hasAdmin: boolean;
  disabled?: boolean;
  canCreate: boolean;
  createDisabledReason?: string;
}) => {
  const queryClient = useQueryClient();
  const formatDate = useProfileDateTimeFormatter();
  const [open, setOpen] = useState(false);
  const [name, setName] = useState('');
  const [expiration, setExpiration] = useState('90');
  const [customExpiration, setCustomExpiration] = useState('');
  const [understandsNever, setUnderstandsNever] = useState(false);
  const [createdToken, setCreatedToken] = useState<string | null>(null);
  const [selectedTokens, setSelectedTokens] = useState<ServiceAccountTokenView[]>([]);
  const [revokeTargets, setRevokeTargets] = useState<ServiceAccountTokenView[]>([]);
  const [selectionVersion, setSelectionVersion] = useState(0);
  const [tokenQuery, setTokenQuery] = useState({ page: 1, pageSize: 10 });
  const [statusReferenceTime] = useState(Date.now);
  const tokens = useRead('listServiceAccountTokens', {
    id: serviceAccountId,
    query: { Page: tokenQuery.page, PageSize: tokenQuery.pageSize },
  });
  const limits = useRead('getServiceAccountLimits');
  const createMutation = useMutate('createServiceAccountToken');
  const revokeMutation = useMutate('revokeServiceAccountToken');
  const defaultLifetimeDays = Number(limits.data?.data.defaultTokenLifetimeDays ?? 90);
  const maximumLifetimeDays = Number(limits.data?.data.maximumTokenLifetimeDays ?? 365);
  const expirationPresets = useMemo(
    () => getTokenExpirationPresets(defaultLifetimeDays, maximumLifetimeDays),
    [defaultLifetimeDays, maximumLifetimeDays],
  );
  const clearCreateMutation = useCallback(() => {
    const cache = queryClient.getMutationCache();
    cache.findAll({ mutationKey: ['createServiceAccountToken'] }).forEach((mutation) => cache.remove(mutation));
  }, [queryClient]);

  useEffect(() => clearCreateMutation, [clearCreateMutation]);

  const closeCreate = () => {
    setOpen(false);
    setName('');
    setExpiration(String(defaultLifetimeDays));
    setCustomExpiration('');
    setUnderstandsNever(false);
    setCreatedToken(null);
    clearCreateMutation();
  };

  const openCreate = () => {
    setExpiration(String(defaultLifetimeDays));
    setOpen(true);
  };

  const createToken = async () => {
    if (expiration === 'custom' && !isValidTokenExpiration(customExpiration, maximumLifetimeDays)) return;
    const expiresAtUtc =
      expiration === 'never'
        ? null
        : expiration === 'custom'
          ? new Date(customExpiration).toISOString()
          : new Date(Date.now() + Number(expiration) * 86_400_000).toISOString();
    const result = await createMutation.mutateAsync({
      id: serviceAccountId,
      data: { name, expiresAtUtc, neverExpires: expiration === 'never' },
    });
    setCreatedToken(result.data.token);
    queryClient.invalidateQueries({ queryKey: ['listServiceAccountTokens'] });
    queryClient.invalidateQueries({ queryKey: ['getServiceAccount', { id: serviceAccountId }] });
  };

  const customExpirationValid =
    expiration !== 'custom' || isValidTokenExpiration(customExpiration, maximumLifetimeDays);

  const revoke = async () => {
    if (revokeTargets.length === 0) return;
    const targets = revokeTargets;
    try {
      for (const [index, token] of targets.entries()) {
        try {
          await revokeMutation.mutateAsync({ id: serviceAccountId, tokenId: token.id });
        } catch (error) {
          setRevokeTargets(targets.slice(index));
          throw error;
        }
      }
    } finally {
      await queryClient.invalidateQueries({ queryKey: ['listServiceAccountTokens'] });
      setSelectedTokens([]);
      setSelectionVersion((current) => current + 1);
    }
  };

  const tokenItems = tokens.data?.data.pagedResult.items ?? [];
  const canRevoke = (token: ServiceAccountTokenView) =>
    !disabled &&
    !token.revokedAtUtc &&
    (!token.expiresAtUtc || new Date(token.expiresAtUtc).getTime() > statusReferenceTime);

  const tokenColumns = useMemo<ColumnDef<ServiceAccountTokenView>[]>(
    () => [
      {
        id: 'select',
        header: ({ table }) => (
          <Checkbox
            checked={table.getIsAllPageRowsSelected() || (table.getIsSomePageRowsSelected() && 'indeterminate')}
            onCheckedChange={(value) => table.toggleAllPageRowsSelected(!!value)}
            aria-label="Select all tokens"
          />
        ),
        cell: ({ row }) => (
          <Checkbox
            checked={row.getIsSelected()}
            disabled={!row.getCanSelect()}
            onCheckedChange={(value) => row.toggleSelected(!!value)}
            aria-label={`Select token ${row.original.name}`}
          />
        ),
        enableSorting: false,
        enableHiding: false,
      },
      {
        accessorKey: 'name',
        header: 'Name',
        cell: ({ row }) => <span className="font-medium">{row.original.name}</span>,
      },
      {
        accessorKey: 'hint',
        header: 'Hint',
        cell: ({ row }) => <span className="font-mono text-xs">{row.original.hint}</span>,
      },
      {
        accessorKey: 'createdAtUtc',
        header: 'Created',
        cell: ({ row }) => <span>{formatDate(row.original.createdAtUtc)}</span>,
      },
      {
        accessorKey: 'createdByName',
        header: 'Created by',
      },
      {
        accessorKey: 'expiresAtUtc',
        header: 'Expires',
        cell: ({ row }) => <span>{row.original.expiresAtUtc ? formatDate(row.original.expiresAtUtc) : 'Never'}</span>,
      },
      {
        accessorKey: 'lastUsedAtUtc',
        header: 'Last used',
        cell: ({ row }) => (
          <span>{row.original.lastUsedAtUtc ? formatDate(row.original.lastUsedAtUtc) : 'Not used'}</span>
        ),
      },
      {
        id: 'status',
        header: 'Status',
        cell: ({ row }) => {
          const expired =
            !!row.original.expiresAtUtc && new Date(row.original.expiresAtUtc).getTime() <= statusReferenceTime;
          if (row.original.revokedAtUtc) return <StateBadge value="information" label="Revoked" />;
          if (expired) return <StateBadge value="warning" label="Expired" />;
          return <StateBadge value="active" label="Active" />;
        },
      },
      {
        id: 'actions',
        cell: ({ row }) => {
          const expired =
            !!row.original.expiresAtUtc && new Date(row.original.expiresAtUtc).getTime() <= statusReferenceTime;
          return !disabled && !row.original.revokedAtUtc && !expired ? (
            <Button
              variant="outline"
              className="shadow-xs"
              size="icon-xs"
              aria-label={`Revoke ${row.original.name}`}
              onClick={() => setRevokeTargets([row.original])}>
              <Trash2 className="size-4 text-destructive" />
            </Button>
          ) : null;
        },
      },
    ],
    [disabled, formatDate, statusReferenceTime],
  );

  const createButton = (
    <Button onClick={openCreate} disabled={disabled || !canCreate}>
      <KeyRound className="size-4" /> Create token
    </Button>
  );

  return (
    <section className="space-y-4">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h3 className="font-semibold">Credentials</h3>
          <p className="text-sm text-muted-foreground">
            Named bearer tokens are shown only once and can be revoked independently.
          </p>
        </div>
        {createDisabledReason ? (
          <Tooltip>
            <TooltipTrigger asChild>
              <span>{createButton}</span>
            </TooltipTrigger>
            <TooltipContent>{createDisabledReason}</TooltipContent>
          </Tooltip>
        ) : (
          createButton
        )}
      </div>
      {hasAdmin && (
        <Alert variant="destructive">
          <AlertTriangle />
          <AlertTitle>Tokens have full administrative access</AlertTitle>
          <AlertDescription>
            Remove the Admin role before creating a token unless this is strictly required.
          </AlertDescription>
        </Alert>
      )}
      <PagedDataTable
        key={selectionVersion}
        columns={tokenColumns}
        data={tokenItems}
        isLoading={tokens.isLoading}
        query={tokenQuery}
        setQuery={(patch) => setTokenQuery((current) => ({ ...current, ...patch }))}
        totalCount={tokens.data?.data.pagedResult.totalCount}
        getRowId={(token) => token.id}
        enableRowSelection={canRevoke}
        onSelectionChange={setSelectedTokens}
      />

      <GenericActionBar
        selectedItems={selectedTokens}
        allItems={tokenItems}
        resource="ServiceAccount"
        resourceLabel="token"
        actions={[
          ({ resources }) => (
            <ActionButton
              title="Revoke"
              icon={<Trash2 className="size-4" />}
              iconPosition="left"
              variant="destructive"
              disabled={disabled || resources.length === 0}
              onClick={() => setRevokeTargets(resources)}
            />
          ),
        ]}
      />

      <Dialog open={open} onOpenChange={(value) => (value ? setOpen(true) : closeCreate())}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{createdToken ? 'Copy your token' : 'Create token'}</DialogTitle>
            <DialogDescription>
              {createdToken
                ? 'Copy this token now. Citadel cannot display it again.'
                : "The token receives this account's current effective permissions."}
            </DialogDescription>
          </DialogHeader>
          {createdToken ? (
            <div className="rounded-md border bg-muted/40 p-3">
              <CopyToClipboard
                textToCopy={createdToken}
                groupClassName="w-full"
                textClassName="flex-1 whitespace-normal break-all font-mono text-xs"
              />
            </div>
          ) : (
            <div className="space-y-4">
              <div className="space-y-1.5">
                <label htmlFor="service-account-token-name" className="text-sm font-medium">
                  Token name
                </label>
                <FieldInput
                  className="max-w-full"
                  id="service-account-token-name"
                  value={name}
                  onChange={setName}
                  placeholder="github-actions-prd"
                />
              </div>
              <div className="space-y-1.5">
                <label htmlFor="service-account-token-expiration" className="text-sm font-medium">
                  Expiration
                </label>
                <Select
                  value={expiration}
                  onValueChange={(value) => {
                    setExpiration(value);
                    setUnderstandsNever(false);
                  }}>
                  <SelectTrigger id="service-account-token-expiration" className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent className="bg-background">
                    {expirationPresets.map((days) => (
                      <SelectItem key={days} value={String(days)}>
                        {days} days
                      </SelectItem>
                    ))}
                    <SelectItem value="custom">Custom</SelectItem>
                    <SelectItem value="never">Never</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              {expiration === 'custom' && (
                <div className="space-y-1.5">
                  <FieldInput type="datetime-local" value={customExpiration} onChange={setCustomExpiration} />
                  {!customExpirationValid && customExpiration && (
                    <p className="text-sm text-destructive">
                      Choose a future date no more than {maximumLifetimeDays} days from now.
                    </p>
                  )}
                </div>
              )}
              {expiration === 'never' && (
                <Alert variant="destructive">
                  <AlertTriangle />
                  <AlertTitle>Non-expiring bearer credential</AlertTitle>
                  <AlertDescription className="space-y-3">
                    <p>
                      This token remains valid until it is revoked, the account is disabled or archived, or licensing
                      suspends Service Accounts.
                      {hasAdmin &&
                        ' It will also have full administrative access while the Admin role remains assigned.'}
                    </p>
                    <label htmlFor="service-account-token-never-confirm" className="flex items-center gap-2">
                      <Checkbox
                        id="service-account-token-never-confirm"
                        checked={understandsNever}
                        onCheckedChange={(value) => setUnderstandsNever(value === true)}
                      />
                      I understand the risk
                    </label>
                  </AlertDescription>
                </Alert>
              )}
            </div>
          )}
          <DialogFooter>
            <Button variant="outline" onClick={closeCreate}>
              {createdToken ? 'Done' : 'Cancel'}
            </Button>
            {!createdToken && (
              <Button
                onClick={createToken}
                disabled={
                  !name.trim() ||
                  !customExpirationValid ||
                  (expiration === 'never' && !understandsNever) ||
                  createMutation.isPending
                }>
                Create token
              </Button>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <ActionWithDialog
        name={revokeTargets.length === 1 ? revokeTargets[0].name : 'Revoke'}
        title={revokeTargets.length === 1 ? 'Revoke token' : 'Revoke tokens'}
        icon={<Trash2 className="size-4" />}
        variant="destructive"
        open={revokeTargets.length > 0}
        onOpenChange={(value) => !value && setRevokeTargets([])}
        renderTrigger={false}
        description="The selected token credentials will stop working immediately. Revocation cannot be undone."
        additional={
          revokeTargets.length > 1 ? (
            <ul className="max-h-75 list-inside list-disc overflow-y-auto bg-accent p-4 text-sm">
              {revokeTargets.map((token) => (
                <li key={token.id}>{token.name}</li>
              ))}
            </ul>
          ) : undefined
        }
        onClick={revoke}
        disabled={revokeMutation.isPending}
      />
    </section>
  );
};

const UsedByPanel = ({ serviceAccountId }: { serviceAccountId: string }) => {
  const usages = useRead('listServiceAccountUsages', { id: serviceAccountId });
  const items = usages.data?.data ?? [];
  const pathFor = (resourceType: ResourceType, id: string) => {
    if (resourceType === ResourceType.AutomationAction) return `/automation/edit/${id}`;
    if (resourceType === ResourceType.BackupPolicy) return `/backup-policies/edit/${id}`;
    return undefined;
  };

  return (
    <section className="space-y-4 border-t pt-6">
      <div>
        <h3 className="font-semibold">Used by</h3>
        <p className="text-sm text-muted-foreground">Resources configured to run with this Service Account.</p>
      </div>
      {items.length === 0 ? (
        <p className="text-sm text-muted-foreground">This Service Account is not referenced by any resource.</p>
      ) : (
        <div className="divide-y rounded-md border">
          {items.map((usage) => {
            const path = pathFor(usage.resourceType, usage.id);
            return (
              <div
                key={`${usage.resourceType}:${usage.id}`}
                className="flex items-center justify-between gap-4 px-3 py-2 text-sm">
                <div className="min-w-0">
                  {path ? (
                    <Link className="font-medium hover:underline" to={path}>
                      {usage.name}
                    </Link>
                  ) : (
                    <span className="font-medium">{usage.name}</span>
                  )}
                  <p className="text-xs text-muted-foreground">{usage.resourceType}</p>
                </div>
                <span className={usage.isActive ? 'text-foreground' : 'text-muted-foreground'}>
                  {usage.isActive ? 'Active' : 'Inactive'}
                </span>
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
};
