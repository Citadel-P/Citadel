import { useCallback, useMemo } from 'react';
import type { AuthorizedRegistryView } from '@/api/generated/api.types';
import { useCollectionRows } from '@/hooks/useCollectionRows';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { collectionGroup } from '@/lib/collection-scope';
import type { RealtimeConnection } from '@/lib/realtime-connection';
import { useRegistryList } from './queries';

export function useRegistriesGroup(tags: string[], enabled: boolean) {
  const args = useMemo(() => ({ query: { includeDisabled: true, ...(tags.length ? { tags } : {}) } }), [tags]);
  const { data, refetch, ...query } = useRegistryList(args, enabled);
  const scope = collectionGroup('registries', tags, undefined, true);
  const reload = useCallback(async () => {
    const result = await refetch();
    return result.isSuccess ? result.data.data.registries : undefined;
  }, [refetch]);
  const [registries, update, onJoinedGroup, onCollectionSnapshot] = useCollectionRows<AuthorizedRegistryView>(
    data?.data.registries,
    scope,
    reload,
  );
  useRealtimeGroup({
    enabled,
    groupName: scope,
    setupEventListeners: useCallback((hub: RealtimeConnection) => hub.on('RegistryInfoUpdated', update), [update]),
    removeEventListeners: useCallback((hub: RealtimeConnection) => hub.off('RegistryInfoUpdated', update), [update]),
    onJoinedGroup,
    onCollectionSnapshot,
  });
  return { registries, capabilities: data?.data.capabilities, refetch, ...query };
}
