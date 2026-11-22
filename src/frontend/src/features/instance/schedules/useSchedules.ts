import { useCallback, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { resourceQueryKey } from '@/api/dispatch';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import type { RealtimeConnection } from '@/lib/realtime-connection';
import { useRead } from '@/lib/hooks';

export function useSchedules(search: string) {
  const [batches, setBatches] = useState<string[]>();
  const client = useQueryClient();
  const query = useRead(
    'getInstanceScheduleGroups',
    { query: { search: search || undefined } },
    { refetchOnMount: 'always', retry: false },
  );
  const refresh = useCallback(() => {
    void client.invalidateQueries({ queryKey: resourceQueryKey('getInstanceScheduleGroups') });
    void client.invalidateQueries({ queryKey: resourceQueryKey('getInstanceSchedules') });
  }, [client]);
  const receive = useCallback(
    (snapshot: { catalogChanged: boolean; activeBatches: string[] }) => {
      setBatches(snapshot.activeBatches);
      if (snapshot.catalogChanged) refresh();
    },
    [refresh],
  );
  const listen = useCallback((c: RealtimeConnection) => c.on('InstanceSchedulesChanged', receive), [receive]);
  const unlisten = useCallback((c: RealtimeConnection) => c.off('InstanceSchedulesChanged', receive), [receive]);
  const start = useCallback(
    (c: RealtimeConnection) => {
      setBatches(undefined);
      void c.invoke('StartInstanceSchedules').catch(refresh);
    },
    [refresh],
  );
  const realtime = useRealtimeGroup({
    groupName: 'instance-schedules',
    setupEventListeners: listen,
    removeEventListeners: unlisten,
    onJoinedGroup: start,
    onSubscriptionError: refresh,
  });
  return {
    query,
    synchronized: realtime.isConnected && batches !== undefined,
    activeBatches: realtime.isConnected ? (batches ?? []) : [],
  };
}
