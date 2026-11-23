import { useCallback } from 'react';
import { useCollectionRows } from './useCollectionRows';
import { useRealtimeGroup } from './useRealtimeGroup';
import type { RealtimeConnection } from '@/lib/realtime-connection';

type CollectionResult<Key extends string, Row> = { data: Record<Key, Row[] | undefined> };

type CollectionSubscription<Key extends string, Row> = {
  groupName: string;
  eventName: string;
  enabled?: boolean;
  field: Key;
  query: {
    data?: CollectionResult<Key, Row>;
    isSuccess: boolean;
    dataUpdatedAt: number;
    refetch: () => Promise<{ isSuccess: boolean; data?: CollectionResult<Key, Row> }>;
  };
  merge?: (incoming: Row, existing: Row) => Row;
};

export function useCollectionSubscription<Key extends string, Row extends { id: string }>({
  groupName,
  eventName,
  enabled,
  field,
  query,
  merge,
}: CollectionSubscription<Key, Row>) {
  const { refetch } = query;
  const reload = useCallback(async () => {
    const result = await refetch();
    return result.isSuccess ? result.data?.data[field] : undefined;
  }, [refetch, field]);
  const { rows, onEvent, onJoinedGroup, onCollectionSnapshot } = useCollectionRows(
    query.data?.data[field],
    groupName,
    reload,
    query.isSuccess ? query.dataUpdatedAt : undefined,
    merge,
  );
  useRealtimeGroup({
    groupName,
    enabled,
    onJoinedGroup,
    onCollectionSnapshot,
    setupEventListeners: useCallback((hub: RealtimeConnection) => hub.on(eventName, onEvent), [eventName, onEvent]),
    removeEventListeners: useCallback((hub: RealtimeConnection) => hub.off(eventName, onEvent), [eventName, onEvent]),
  });
  return rows;
}
