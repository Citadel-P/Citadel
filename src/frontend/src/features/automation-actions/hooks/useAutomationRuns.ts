import { useCallback, useMemo } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import type { AutomationRunView } from '@/api/generated/api.types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import type { RealtimeConnection } from '@/lib/realtime-connection';

export function useAutomationRuns(actionId: string) {
  const queryClient = useQueryClient();
  const args = useMemo(() => ({ id: actionId, query: { limit: 50 } }), [actionId]);
  const query = useRead('listAutomationActionRuns', args);
  const receiveWindow = useCallback(
    (snapshot: { actionId: string; ids: string[] }) => {
      if (snapshot.actionId !== actionId) return;
      const filter = { queryKey: ['listAutomationActionRuns', args], exact: true };
      void queryClient.cancelQueries(filter);
      queryClient.setQueryData<{ data: { runs: AutomationRunView[] } }>(filter.queryKey, (current) => ({
        data: { runs: (current?.data.runs ?? []).filter((run) => snapshot.ids.includes(run.id)) },
      }));
    },
    [actionId, args, queryClient],
  );
  const receive = useCallback(
    (run: AutomationRunView, operation: string) => {
      if (run.actionId !== actionId || operation === 'delete') return;
      const filter = { queryKey: ['listAutomationActionRuns', args], exact: true };
      // Cancel the older GET before updating the authoritative live window.
      void queryClient.cancelQueries(filter);
      queryClient.setQueryData<{ data: { runs: AutomationRunView[] } }>(filter.queryKey, (current) => ({
        data: {
          runs: [...(current?.data.runs ?? []).filter((item) => item.id !== run.id), run]
            .sort((a, b) => String(b.queuedAt).localeCompare(String(a.queuedAt)) || b.id.localeCompare(a.id))
            .slice(0, 50),
        },
      }));
    },
    [actionId, args, queryClient],
  );
  const setupEventListeners = useCallback(
    (connection: RealtimeConnection) => {
      connection.on('AutomationRunWindow', receiveWindow);
      connection.on('AutomationRunInfoUpdated', receive);
    },
    [receiveWindow, receive],
  );
  const removeEventListeners = useCallback(
    (connection: RealtimeConnection) => {
      connection.off('AutomationRunWindow', receiveWindow);
      connection.off('AutomationRunInfoUpdated', receive);
    },
    [receiveWindow, receive],
  );
  useRealtimeGroup({
    groupName: `automation-runs:${actionId}`,
    setupEventListeners,
    removeEventListeners,
  });
  return query;
}
