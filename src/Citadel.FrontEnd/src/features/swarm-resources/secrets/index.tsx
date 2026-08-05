import { RegularResourceComponents } from '@/pages/types';
import { ActionBar } from '@/components/custom/action-bar';
import { KeyRound } from 'lucide-react';
import { useSecretsGroup } from './hooks/useSecretsGroup';
import { SecretsTable } from './table';
import { SecretDropdownActions, SecretGroupActions } from './actions';

export const SecretComponents: RegularResourceComponents = {
  Icon: KeyRound,
  header: {
    title: 'Secrets',
    subtitle: 'Secret metadata only. Citadel never reads or returns secret data.',
    showAdd: true,
    showSearch: true,
  },
  Content: ({ items, actions, isLoading }) => <SecretsTable items={items} actions={actions} isLoading={isLoading} />,
  DropdownActions: SecretDropdownActions,
  GroupActions: ({ items }) => <ActionBar type="Secret" items={items} actions={Object.values(SecretGroupActions)} />,
  useData: (platformId) => {
    const { items, capabilities, isLoading } = useSecretsGroup(platformId);
    return { items, isLoading, capabilities };
  },
  filterItems: (items, search) => {
    const value = search.trim().toLowerCase();
    return value
      ? items.filter((item) => item.name?.toLowerCase().includes(value) || item.id?.toLowerCase().includes(value))
      : items;
  },
};
