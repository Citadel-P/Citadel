import { useMemo } from 'react';
import type { ManagedSwarmServiceView } from '@/api/generated/api.types';
import { useCollectionSubscription } from '@/hooks/useCollectionSubscription';
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
  const query = useRead('listManagedSwarmServices', args, { enabled: filterReady });
  const services = useCollectionSubscription({
    groupName: scope,
    eventName: 'ManagedSwarmServiceUpdated',
    enabled: filterReady,
    field: 'swarmServices',
    query,
    merge,
  });
  return { ...query, services, capabilities: query.data?.data.capabilities };
};
