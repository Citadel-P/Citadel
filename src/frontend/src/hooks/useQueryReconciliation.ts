import { replaceEqualDeep, useQueryClient } from '@tanstack/react-query';
import { useCallback, useContext } from 'react';
import { RealtimeContext } from '@/lib/context/realtime-context';
import type { KnownResourceName } from '@/api/types';

// These lists keep live rows separately. A fallback GET must replace them even
// when its rows are identical to the last HTTP baseline.
export function useCollectionStructuralSharing(group: string) {
  const isGroupLive = useContext(RealtimeContext)?.groups?.isGroupLive;
  return useCallback(
    (previous: unknown, next: unknown) => (isGroupLive?.(group) ? replaceEqualDeep(previous, next) : next),
    [group, isGroupLive],
  );
}

// Check membership when the mutation settles. The socket or the subscription may have changed during the request.
export function useQueryReconciliation() {
  const client = useQueryClient();
  const isGroupLive = useContext(RealtimeContext)?.groups?.isGroupLive;
  return useCallback(
    async (group: string, operation: KnownResourceName, args?: object) => {
      if (isGroupLive?.(group)) return;
      const queryKey = args ? [operation, args] : [operation];
      await client.cancelQueries({ queryKey, fetchStatus: 'fetching' });
      await client.invalidateQueries({ queryKey });
    },
    [client, isGroupLive],
  );
}

export function useResourceReconciliation(
  listGroup: string,
  listOperation: KnownResourceName,
  detailGroup: string,
  detailOperation: KnownResourceName,
) {
  const reconcile = useQueryReconciliation();
  return useCallback(
    async (id?: string) => {
      await Promise.all([
        reconcile(listGroup, listOperation),
        id && reconcile(`${detailGroup}:${id}`, detailOperation, { id }),
      ]);
    },
    [reconcile, listGroup, listOperation, detailGroup, detailOperation],
  );
}
