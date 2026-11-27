import { useCallback, useMemo } from 'react';
import { hashKey, useQueryClient } from '@tanstack/react-query';
import { resourceQueryKey } from '@/api/dispatch';
import type { UseReadArgs } from '@/api/types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import type { RealtimeConnection } from '@/lib/realtime-connection';
import { useRead } from '@/lib/hooks';

type Projection = 'getDashboard' | 'getDashboardFleet' | 'listDashboardSources' | 'listRecentActivities';

// Identical projections share a group and cache entry, including when two widgets use the same source.
export function useDashboardRead<K extends Projection>(
  resource: K,
  args: UseReadArgs<K>,
  { enabled = true, staleTime = 15000, retry = false } = {},
) {
  const client = useQueryClient();
  const identity = hashKey(resourceQueryKey(resource, args));
  const key = useMemo(
    () => JSON.parse(identity) as [Projection, { id?: string; query?: { tags?: string[] } }],
    [identity],
  );
  const groupName = useMemo(() => {
    const { tags = [], ...query } = key[1].query ?? {};
    return JSON.stringify({
      group: 'dashboard',
      projection: key[0],
      tags,
      query: key[0] === 'getDashboard' ? { id: key[1].id } : query,
    });
  }, [key]);
  const receive = useCallback(
    (data: unknown) => {
      const filter = { queryKey: key, exact: true };
      void client.cancelQueries(filter);
      client.setQueryData(key, { data });
      // HTTP supplies the normal resource error if the saved board is gone or access was revoked.
      if (data === null) void client.invalidateQueries(filter);
    },
    [client, key],
  );
  const listen = useCallback((c: RealtimeConnection) => c.on('DashboardSnapshot', receive), [receive]);
  const unlisten = useCallback((c: RealtimeConnection) => c.off('DashboardSnapshot', receive), [receive]);
  const { isConnected } = useRealtimeGroup({
    groupName,
    enabled,
    setupEventListeners: listen,
    removeEventListeners: unlisten,
  });
  return useRead(resource, args, { enabled, staleTime, retry, refetchInterval: isConnected ? false : 60000 });
}
