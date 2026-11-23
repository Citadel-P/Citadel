import { collectionListeners } from '@/lib/collection-scope';
import { useEffect, useRef, useState } from 'react';
import { GROUP_CAPACITY_ERROR, RealtimeConnection, RealtimeConnectionState } from '@/lib/realtime-connection';
import { useRealtimeContext } from '@/lib/context/realtime-context';

type UseRealtimeGroupOpts = {
  groupName?: string | readonly string[];
  setupEventListeners: (hub: RealtimeConnection) => void;
  removeEventListeners?: (hub: RealtimeConnection) => void;
  onCollectionSnapshot?: (active: boolean) => void;
  onSubscriptionError?: (error: unknown) => void;
  onJoinedGroup?: (hub: RealtimeConnection) => void;
  skip?: boolean;
  enabled?: boolean;
};

export const useRealtimeGroup = ({
  groupName,
  setupEventListeners,
  removeEventListeners,
  onJoinedGroup,
  onSubscriptionError,
  onCollectionSnapshot,
  skip,
  enabled = true,
}: UseRealtimeGroupOpts) => {
  const { groups } = useRealtimeContext();
  const connection = groups?.connection;
  const connectionState = groups?.connectionState ?? RealtimeConnectionState.Disconnected;
  const joinGroup = groups?.joinGroup;
  const leaveGroup = groups?.leaveGroup;
  const [isLoading, setIsLoading] = useState(false);
  const [subscription, setSubscription] = useState<{ connection: RealtimeConnection; groupName: typeof groupName }>();

  const onJoinedRef = useRef(onJoinedGroup);
  const onErrorRef = useRef(onSubscriptionError);

  const loadingRef = useRef(false);

  useEffect(() => {
    onJoinedRef.current = onJoinedGroup;
    onErrorRef.current = onSubscriptionError;
  }, [onJoinedGroup, onSubscriptionError]);

  const state = connectionState;
  const limited =
    !skip &&
    enabled &&
    (typeof groupName === 'string' ? [groupName] : groupName)?.some((name) => groups?.capacityGroups.has(name));
  const live =
    !!subscription &&
    subscription.groupName === groupName &&
    subscription.connection === connection &&
    !skip &&
    enabled &&
    !limited &&
    state === RealtimeConnectionState.Connected;

  useEffect(() => {
    if (limited) {
      onCollectionSnapshot?.(false);
      onErrorRef.current?.(new Error(GROUP_CAPACITY_ERROR));
    }
  }, [limited, onCollectionSnapshot]);

  useEffect(() => {
    if (live) onJoinedRef.current?.(subscription.connection);
  }, [subscription, live]);

  useEffect(() => {
    const setLoading = (val: boolean) => {
      if (loadingRef.current !== val) {
        loadingRef.current = val;
        setIsLoading(val);
      }
    };
    const groupNames = typeof groupName === 'string' ? [groupName] : (groupName ?? []);
    if (
      skip ||
      !enabled ||
      !groupNames.length ||
      !connection ||
      !joinGroup ||
      !leaveGroup ||
      state !== RealtimeConnectionState.Connected
    ) {
      setLoading(false);
      // A disconnected or disabled group releases its subscription.
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setSubscription(undefined);
      return;
    }

    const joined: string[] = [];
    let releasedCount = 0;
    const listenerConnection =
      typeof groupName === 'string' && groupName.startsWith('{')
        ? collectionListeners(connection, groupName)
        : connection;
    const scopedSnapshot =
      typeof groupName === 'string' && groupName.startsWith('{') ? onCollectionSnapshot : undefined;
    const startSnapshot = () => scopedSnapshot?.(true);
    let listenersRemoved = false;
    let disposed = false;

    const removeListeners = () => {
      if (listenersRemoved) return;
      listenersRemoved = true;
      if (scopedSnapshot) {
        listenerConnection.off('CollectionSnapshotStarted', startSnapshot);
        scopedSnapshot(false);
      }
      removeEventListeners?.(listenerConnection);
    };

    const leaveJoinedGroups = async () => {
      const groups = joined.slice(releasedCount);
      releasedCount = joined.length;
      await Promise.all(groups.map((currentGroupName) => leaveGroup(currentGroupName)));
    };

    const run = async () => {
      try {
        setLoading(true);
        if (scopedSnapshot) listenerConnection.on('CollectionSnapshotStarted', startSnapshot);
        setupEventListeners(listenerConnection);
        for (const currentGroupName of groupNames) {
          await joinGroup(currentGroupName);
          joined.push(currentGroupName);
          if (disposed) break;
        }

        if (disposed) {
          await leaveJoinedGroups();
          removeListeners();
          return;
        }

        setSubscription({ connection, groupName });
        setLoading(false);
      } catch (err) {
        if (!disposed) onErrorRef.current?.(err);
        console.error('[Realtime] join failed', err);
        await leaveJoinedGroups();
        removeListeners();
        if (!disposed) {
          setSubscription(undefined);
          setLoading(false);
        }
      }
    };

    run();

    return () => {
      disposed = true;
      removeListeners();
      leaveJoinedGroups().catch(console.warn);
    };
  }, [
    groupName,
    skip,
    enabled,
    state,
    connection,
    joinGroup,
    leaveGroup,
    setupEventListeners,
    removeEventListeners,
    onCollectionSnapshot,
  ]);

  return { isLoading, isConnected: live, isCapacityLimited: !!limited };
};
