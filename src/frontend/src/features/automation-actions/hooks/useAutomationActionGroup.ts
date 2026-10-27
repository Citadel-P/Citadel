import { useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import type { AuthorizedAction } from '@/api/generated/api.types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import type { RealtimeConnection } from '@/lib/realtime-connection';

export function useAutomationActionGroup(id: string) {
  const queryClient = useQueryClient();
  const query = useRead('getAutomationAction', { id });
  const receive = useCallback(
    (action: AuthorizedAction, operation: string) => {
      if (action.id !== id || operation === 'delete') return;
      const filter = { queryKey: ['getAutomationAction', { id }], exact: true };
      void queryClient.cancelQueries(filter);
      queryClient.setQueryData(filter.queryKey, { data: action });
    },
    [id, queryClient],
  );
  const setupEventListeners = useCallback(
    (connection: RealtimeConnection) => connection.on('AutomationActionInfoUpdated', receive),
    [receive],
  );
  const removeEventListeners = useCallback(
    (connection: RealtimeConnection) => connection.off('AutomationActionInfoUpdated', receive),
    [receive],
  );
  useRealtimeGroup({ groupName: `automation-action:${id}`, setupEventListeners, removeEventListeners });
  return query;
}
