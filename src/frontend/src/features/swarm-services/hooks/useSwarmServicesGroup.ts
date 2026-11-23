import { useCallback, useMemo } from 'react';
import type { RealtimeConnection } from '@/lib/realtime-connection';
import type { ManagedSwarmServiceView } from '@/api/generated/api.types';
import { useCollectionRows } from '@/hooks/useCollectionRows';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { collectionGroup } from '@/lib/collection-scope';
import { useRead } from '@/lib/hooks';
import { useResourceTagFilter } from '@/features/tags/components';
import { useResourcePlatformFilter } from '@/features/platforms/platform-filter';
import { normalizeManagedSwarmService } from './useSwarmServiceGroup';

const EMPTY_TAG_NAMES: string[] = [];
const merge = (incoming: ManagedSwarmServiceView, existing: ManagedSwarmServiceView) =>
  normalizeManagedSwarmService(existing, incoming);

export const useSwarmServicesGroup = (platformIdOverride?: string) => {
  const { selectedTagNames, filterReady } = useResourceTagFilter(!platformIdOverride);
  const { selectedPlatformId } = useResourcePlatformFilter();
  const platformId = platformIdOverride ?? selectedPlatformId;
  const tags = platformIdOverride ? EMPTY_TAG_NAMES : selectedTagNames;
  const scope = collectionGroup('managed-swarm-services', tags, platformId);
  const args = useMemo(() => {
    const query = { ...(tags.length ? { tags } : {}), ...(platformId ? { platformId } : {}) };
    return Object.keys(query).length ? { query } : undefined;
  }, [platformId, tags]);
  const { data, refetch, ...query } = useRead('listManagedSwarmServices', args, { enabled: filterReady });
  const reload = useCallback(async () => {
    const result = await refetch();
    return result.isSuccess ? result.data.data.swarmServices : undefined;
  }, [refetch]);
  const [services, update, onJoinedGroup, onCollectionSnapshot] = useCollectionRows(
    data?.data.swarmServices,
    scope,
    reload,
    merge,
  );
  useRealtimeGroup({
    enabled: filterReady,
    groupName: scope,
    setupEventListeners: useCallback(
      (hub: RealtimeConnection) => hub.on('ManagedSwarmServiceUpdated', update),
      [update],
    ),
    removeEventListeners: useCallback(
      (hub: RealtimeConnection) => hub.off('ManagedSwarmServiceUpdated', update),
      [update],
    ),
    onJoinedGroup,
    onCollectionSnapshot,
  });
  return { ...query, refetch, services, capabilities: data?.data.capabilities };
};
