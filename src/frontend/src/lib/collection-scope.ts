import type { QueryClient } from '@tanstack/react-query';
import type { RealtimeConnection } from './realtime-connection';

export function collectionGroup(group: string, tags: readonly string[], platformId?: string, includeDisabled = false) {
  if (!tags.length && !platformId && !includeDisabled) return group;
  return JSON.stringify({
    group,
    tags,
    ...(platformId ? { platformId } : {}),
    ...(includeDisabled ? { includeDisabled } : {}),
  });
}

// Keep scope identity case-sensitive. Only the protocol event names are case-insensitive.
export const collectionEventKey = (scope: string, event: string) => JSON.stringify([scope, event.toLowerCase()]);

export function collectionListeners(connection: RealtimeConnection, scope: string): RealtimeConnection {
  return {
    ...connection,
    on: (name, handler) => connection.on(collectionEventKey(scope, name), handler),
    off: (name, handler) => connection.off(collectionEventKey(scope, name), handler),
  };
}

export function hasQueryTagFilter(queryKey: readonly unknown[]) {
  const tags = (queryKey[1] as { query?: { tags?: unknown } } | undefined)?.query?.tags;
  return Array.isArray(tags) && tags.length > 0;
}

export async function reconcileTaggedEventPages(
  client: QueryClient,
  resource: 'listAlertEvents' | 'listRecentActivities' | 'listDashboardSources' | 'getUpcomingSchedules',
) {
  // Paused dashboards retain their last rendered values after removing observers.
  // Invalidate those snapshots without starting background reads.
  await client.invalidateQueries({
    queryKey: [resource],
    type: 'inactive',
    predicate: ({ queryKey }) => hasQueryTagFilter(queryKey),
    refetchType: 'none',
  });
  const filter = {
    queryKey: [resource],
    type: 'active' as const,
    predicate: ({ queryKey }: { queryKey: readonly unknown[] }) => hasQueryTagFilter(queryKey),
  };
  // A read begun before the notification can still carry the old membership.
  const pending = { ...filter, fetchStatus: 'fetching' as const };
  if (client.isFetching(pending)) await client.refetchQueries(pending, { cancelRefetch: false });
  await client.invalidateQueries(filter, { cancelRefetch: false });
}
