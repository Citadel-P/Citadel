import { OidcProviderInput, OidcProviderView, UpdateOidcProviderInput } from '@/api/generated/api.types';
import {
  defineField,
  defineGroupField,
  defineSection,
  FieldInput,
  FieldSwitch,
  FieldTextArea,
  FormShell,
} from '@/components/custom/form-builder';
import { Button } from '@/components/ui/button';
import { MultiSelect } from '@/components/ui/multi-select';
import { useMutate, useRead, useSaveResource } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { RefreshCw } from 'lucide-react';
import { useCallback, useMemo, useState } from 'react';
import { useParams } from 'react-router';
import { toast } from 'sonner';

type OidcProviderFormValue = Omit<OidcProviderInput, 'defaultRoleId'> & {
  id?: string;
  defaultRoleId: string;
  hasClientSecret?: boolean;
};

const emptyProvider = (): OidcProviderFormValue => ({
  name: '',
  description: '',
  displayName: '',
  issuer: '',
  clientId: '',
  clientSecret: '',
  scopes: 'openid profile email',
  enabled: true,
  autoProvisionUsers: false,
  allowEmailAutoLink: false,
  requireEmailVerified: true,
  allowedEmailDomains: '',
  requiredClaimName: '',
  requiredClaimValues: '',
  defaultRoleId: '',
  hasClientSecret: false,
});

export function OidcProviderForm({ mode, resource }: { mode: 'add' | 'edit'; resource?: OidcProviderView }) {
  const { id } = useParams();
  const queryClient = useQueryClient();
  const createProvider = useMutate('createOidcProvider');
  const updateProvider = useMutate('updateOidcProvider');
  const testDiscovery = useMutate('testOidcDiscovery');
  const [update, setUpdate] = useState<Partial<OidcProviderFormValue>>({});
  const { data: rolesData, isLoading: rolesLoading } = useRead('listRoles');

  const original = useMemo(() => toFormValue(resource), [resource]);
  const autoProvisionUsers = update.autoProvisionUsers ?? original.autoProvisionUsers;
  const roleOptions = useMemo(
    () => (rolesData?.data?.roles ?? []).map((role) => ({ label: role.name, value: role.id })),
    [rolesData],
  );

  const { save: handleSave, isPending } = useSaveResource<OidcProviderFormValue, any>({
    mode,
    basePath: 'oidc-providers',
    entityName: 'OIDC provider',
    onCreate: (payload) => createProvider.mutateAsync({ data: toCreateInput(payload) } as any),
    onUpdate: (payload) => updateProvider.mutateAsync({ id: id!, data: toUpdateInput(payload) } as any),
    onRefresh: () => {
      localStorage.removeItem(`oidc-provider:${id ?? 'new'}`);
      queryClient.invalidateQueries({ queryKey: ['getOidcProvider', { id }] });
      queryClient.invalidateQueries({ queryKey: ['listOidcProviders'] });
      queryClient.invalidateQueries({ queryKey: ['listOidcLoginProviders'] });
    },
  });

  const testCurrentIssuer = useCallback(async () => {
    const issuer = (update.issuer ?? original.issuer).trim();
    if (!issuer) return;

    try {
      const result = await testDiscovery.mutateAsync({ data: { issuer } } as any);
      toast.success(`Discovery OK: ${result.data.issuer}`);
    } catch {
      /*Nope*/
    }
  }, [original.issuer, testDiscovery, update.issuer]);

  const schema = useMemo(
    () => ({
      Provider: defineSection<OidcProviderFormValue>({
        title: 'Provider',
        items: [
          defineGroupField<OidcProviderFormValue>({
            id: 'identity',
            label: 'Identity',
            items: [
              ...(mode === 'add'
                ? [
                    defineField<OidcProviderFormValue, 'name'>({
                      key: 'name',
                      persistDraft: true,
                      label: 'Name',
                      required: true,
                      description: 'Stable internal identifier for this provider.',
                      render: (value, set) => (
                        <FieldInput value={value ?? ''} placeholder="google" onChange={(name) => set({ name })} />
                      ),
                    }),
                    defineField<OidcProviderFormValue, 'description'>({
                      key: 'description',
                      persistDraft: true,
                      label: 'Description',
                      description: 'Optional note shown in the provider header.',
                      render: (value, set) => (
                        <FieldTextArea
                          value={value ?? ''}
                          placeholder="Company single sign-on"
                          onChange={(description) => set({ description })}
                        />
                      ),
                    }),
                  ]
                : []),
              defineField({
                key: 'displayName',
                label: 'Display Name',
                required: true,
                description: 'Name shown to users on the sign-in screen.',
                render: (value, set) => (
                  <FieldInput
                    value={value ?? ''}
                    placeholder="Google"
                    onChange={(displayName) => set({ displayName })}
                  />
                ),
              }),
              defineField({
                key: 'enabled',
                label: 'Enabled',
                render: (value, set) => (
                  <FieldSwitch checked={value ?? false} id="oidc-enabled" onChange={(enabled) => set({ enabled })} />
                ),
              }),
              defineField({
                key: 'issuer',
                label: 'Issuer',
                required: true,
                description: 'OIDC issuer URL used for discovery.',
                render: (value, set) => (
                  <div className="flex max-w-160 flex-col gap-2 sm:flex-row">
                    <FieldInput
                      className="max-w-full flex-1"
                      value={value ?? ''}
                      placeholder="https://accounts.google.com"
                      onChange={(issuer) => set({ issuer })}
                    />
                    <Button
                      type="button"
                      variant="outline"
                      onClick={testCurrentIssuer}
                      disabled={testDiscovery.isPending || !String(value ?? '').trim()}>
                      <RefreshCw className="size-3.5" />
                      Test
                    </Button>
                  </div>
                ),
              }),
            ],
          }),
          defineGroupField<OidcProviderFormValue>({
            id: 'client',
            label: 'Client',
            items: [
              defineField({
                key: 'clientId',
                label: 'Client ID',
                required: true,
                render: (value, set) => (
                  <FieldInput value={value ?? ''} placeholder="client-id" onChange={(clientId) => set({ clientId })} />
                ),
              }),
              defineField({
                key: 'clientSecret',
                label: resource?.hasClientSecret ? 'Client Secret (leave blank to keep)' : 'Client Secret',
                required: mode === 'add',
                render: (value, set) => (
                  <FieldInput
                    type="password"
                    value={value ?? ''}
                    placeholder={resource?.hasClientSecret ? 'Stored secret' : 'client-secret'}
                    onChange={(clientSecret) => set({ clientSecret })}
                  />
                ),
              }),
              defineField({
                key: 'scopes',
                label: 'Scopes',
                required: true,
                render: (value, set) => (
                  <FieldInput
                    value={value ?? ''}
                    placeholder="openid profile email"
                    onChange={(scopes) => set({ scopes })}
                  />
                ),
              }),
            ],
          }),
        ],
      }),
      Access: defineSection<OidcProviderFormValue>({
        title: 'Access',
        items: [
          defineGroupField<OidcProviderFormValue>({
            id: 'access-settings',
            label: 'Settings',
            items: [
              defineField({
                key: 'requireEmailVerified',
                label: 'Require Verified Email',
                description:
                  'Reject users whose OIDC email_verified claim is missing or false. This is checked before linking or provisioning.',
                render: (value, set) => (
                  <FieldSwitch
                    checked={value ?? false}
                    id="oidc-require-email-verified"
                    onChange={(requireEmailVerified) => set({ requireEmailVerified })}
                  />
                ),
              }),
              defineField({
                key: 'autoProvisionUsers',
                label: 'Auto Provision Users',
                description:
                  'Create a Citadel user on first successful OIDC sign-in when no linked or matching user exists.',
                render: (value, set) => (
                  <FieldSwitch
                    checked={value ?? false}
                    id="oidc-auto-provision-users"
                    onChange={(autoProvisionUsers) => set({ autoProvisionUsers })}
                  />
                ),
              }),
              defineField<OidcProviderFormValue, 'defaultRoleId'>({
                key: 'defaultRoleId',
                label: 'Default Role',
                description: 'Role assigned to new auto-provisioned users. Leave empty to create users without access.',
                disabled: !autoProvisionUsers,
                render: (value, set) => (
                  <div className="max-w-100">
                    <MultiSelect
                      options={roleOptions}
                      defaultValue={value ? [value] : []}
                      onValueChange={(values) => set({ defaultRoleId: values.at(-1) ?? '' })}
                      placeholder={rolesLoading ? 'Loading...' : 'Select a role...'}
                      disabled={rolesLoading || !autoProvisionUsers}
                      maxCount={1}
                      hideSelectAll={true}
                      closeOnSelect={true}
                      animation={0}
                      resetOnDefaultValueChange={true}
                    />
                  </div>
                ),
              }),
              defineField({
                key: 'allowEmailAutoLink',
                label: 'Allow Email Auto Link',
                description:
                  'Link a first-time OIDC identity to an existing Citadel user when the email address matches.',
                render: (value, set) => (
                  <FieldSwitch
                    checked={value ?? false}
                    id="oidc-allow-email-auto-link"
                    onChange={(allowEmailAutoLink) => set({ allowEmailAutoLink })}
                  />
                ),
              }),
            ],
          }),
        ],
      }),
      Restrictions: defineSection<OidcProviderFormValue>({
        title: 'Restrictions',
        items: [
          defineGroupField<OidcProviderFormValue>({
            id: 'claim-restrictions',
            label: 'Claims',
            items: [
              defineField({
                key: 'allowedEmailDomains',
                label: 'Allowed Email Domains',
                description: 'Comma-separated list. Leave blank to allow any domain.',
                render: (value, set) => (
                  <FieldInput
                    value={value ?? ''}
                    placeholder="example.com, admin.example.com"
                    onChange={(allowedEmailDomains) => set({ allowedEmailDomains })}
                  />
                ),
              }),
              defineField({
                key: 'requiredClaimName',
                label: 'Required Claim Name',
                render: (value, set) => (
                  <FieldInput
                    value={value ?? ''}
                    placeholder="hd"
                    onChange={(requiredClaimName) => set({ requiredClaimName })}
                  />
                ),
              }),
              defineField({
                key: 'requiredClaimValues',
                label: 'Required Claim Values',
                description: 'Comma-separated list. Used only when a required claim name is set.',
                render: (value, set) => (
                  <FieldTextArea
                    value={value ?? ''}
                    placeholder="value1, value2"
                    onChange={(requiredClaimValues) => set({ requiredClaimValues })}
                  />
                ),
              }),
            ],
          }),
        ],
      }),
    }),
    [
      autoProvisionUsers,
      mode,
      resource?.hasClientSecret,
      roleOptions,
      rolesLoading,
      testCurrentIssuer,
      testDiscovery.isPending,
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
      draftKey={`oidc-provider:${id ?? 'new'}`}
      draftVersion={1}
    />
  );
}

function toFormValue(provider?: OidcProviderView): OidcProviderFormValue {
  if (!provider) return emptyProvider();

  return {
    id: provider.id,
    name: provider.name,
    description: provider.description ?? '',
    displayName: provider.displayName,
    issuer: provider.issuer,
    clientId: provider.clientId,
    clientSecret: '',
    scopes: provider.scopes,
    enabled: provider.enabled,
    autoProvisionUsers: provider.autoProvisionUsers,
    allowEmailAutoLink: provider.allowEmailAutoLink,
    requireEmailVerified: provider.requireEmailVerified,
    allowedEmailDomains: provider.allowedEmailDomains ?? '',
    requiredClaimName: provider.requiredClaimName ?? '',
    requiredClaimValues: provider.requiredClaimValues ?? '',
    defaultRoleId: provider.defaultRoleId ?? '',
    hasClientSecret: provider.hasClientSecret,
  };
}

function toCreateInput(value: OidcProviderFormValue): OidcProviderInput {
  return {
    ...normalize(value),
    clientSecret: value.clientSecret?.trim() || null,
  };
}

function toUpdateInput(value: OidcProviderFormValue): UpdateOidcProviderInput {
  return {
    ...normalize(value),
    clientSecret: value.clientSecret?.trim() || null,
  };
}

function normalize(value: OidcProviderFormValue): Omit<OidcProviderInput, 'clientSecret'> {
  return {
    name: value.name.trim(),
    description: value.description?.trim() || null,
    displayName: value.displayName.trim(),
    issuer: value.issuer.trim(),
    clientId: value.clientId.trim(),
    scopes: value.scopes?.trim() || null,
    enabled: value.enabled,
    autoProvisionUsers: value.autoProvisionUsers,
    allowEmailAutoLink: value.allowEmailAutoLink,
    requireEmailVerified: value.requireEmailVerified,
    allowedEmailDomains: value.allowedEmailDomains?.trim() || null,
    requiredClaimName: value.requiredClaimName?.trim() || null,
    requiredClaimValues: value.requiredClaimValues?.trim() || null,
    defaultRoleId: value.defaultRoleId?.trim() || null,
  };
}
