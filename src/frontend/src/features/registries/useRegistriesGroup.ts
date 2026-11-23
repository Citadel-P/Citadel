import { useMemo } from 'react';
import { useCollectionSubscription } from '@/hooks/useCollectionSubscription';
import { collectionGroup } from '@/lib/collection-scope';
import { useRegistryList } from './queries';

export function useRegistriesGroup(tags: string[], enabled: boolean) {
  const args = useMemo(() => ({ query: { includeDisabled: true, ...(tags.length ? { tags } : {}) } }), [tags]);
  const query = useRegistryList(args, enabled);
  const scope = collectionGroup('registries', tags, undefined, true);
  const registries = useCollectionSubscription({
    groupName: scope,
    eventName: 'RegistryInfoUpdated',
    enabled,
    field: 'registries',
    query,
  });
  return { registries, capabilities: query.data?.data.capabilities, ...query };
}
