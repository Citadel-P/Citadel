import { RegularResourceComponents } from '@/pages/types';
import { KeyRound } from 'lucide-react';
import { useSecretsGroup } from './hooks/useSecretsGroup';
import { SecretsTable } from './table';

export const SecretComponents: RegularResourceComponents = {
  Icon: KeyRound,
  header: {
    title: 'Secrets',
    subtitle: 'Secret metadata only. Citadel never reads or returns secret data.',
    showAdd: false,
    showSearch: true,
  },
  Content: ({ items, isLoading }) => <SecretsTable items={items} isLoading={isLoading} />,
  useData: (platformId) => {
    const { items, isLoading } = useSecretsGroup(platformId);
    return { items, isLoading, capabilities: undefined };
  },
  filterItems: (items, search) => {
    const value = search.trim().toLowerCase();
    return value
      ? items.filter((item) => item.name?.toLowerCase().includes(value) || item.id?.toLowerCase().includes(value))
      : items;
  },
};
