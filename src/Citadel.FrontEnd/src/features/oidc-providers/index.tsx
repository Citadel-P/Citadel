import { OidcProviderView, ResourceCapabilities } from '@/api/generated/api.types';
import { ActionBar } from '@/components/custom/action-bar';
import { CitadelIcons } from '@/lib/icons';
import { useRead } from '@/lib/hooks';
import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { OidcProviderDropdownActions, OidcProviderGroupActions } from './actions';
import { OidcProvidersTable } from './table';

const EMPTY_CAPABILITIES: ResourceCapabilities = { canRead: true, canWrite: true, canExecute: false };
const EMPTY_OIDC_PROVIDERS: never[] = [];

export const OidcProviderComponents: RequiredComponents<OidcProviderView> = {
  Icon: CitadelIcons.OidcProvider,
  Content: ({ items, actions, isLoading }) => (
    <OidcProvidersTable items={items} actions={actions} isLoading={isLoading} />
  ),
  header: {
    title: 'OIDC Providers',
    subtitle: 'Manage external OpenID Connect sign-in providers.',
    showSearch: true,
    showAdd: true,
  },
  DropdownActions: OidcProviderDropdownActions,
  GroupActions: ({ items }) => (
    <ActionBar type="OidcProvider" items={items} actions={Object.values(OidcProviderGroupActions)} />
  ),
  useData(): ResourceDataHookResult<OidcProviderView> {
    const { data, isLoading } = useRead('listOidcProviders');
    return { items: data?.data.providers ?? EMPTY_OIDC_PROVIDERS, isLoading, capabilities: EMPTY_CAPABILITIES };
  },
  filterItems: (items, search) => {
    if (!search.trim()) return items;
    const s = search.toLowerCase();
    return items.filter(
      (provider) =>
        provider.name.toLowerCase().includes(s) ||
        provider.displayName.toLowerCase().includes(s) ||
        provider.issuer.toLowerCase().includes(s) ||
        provider.clientId.toLowerCase().includes(s),
    );
  },
};
