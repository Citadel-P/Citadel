import { useEffect, useRef, useState } from 'react';
import { GROUP_CAPACITY_ERROR, RealtimeConnection, RealtimeConnectionState } from '@/lib/realtime-connection';
import { useRealtimeContext } from '@/lib/context/realtime-context';

type UseRealtimeGroupOpts = {
  groupName?: string | readonly string[];
  setupEventListeners: (hub: RealtimeConnection) => void;
  removeEventListeners?: (hub: RealtimeConnection) => void;
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
  skip,
  enabled = true,
}: UseRealtimeGroupOpts) => {
  const { groups } = useRealtimeContext();
  const connection = groups?.connection;
  const connectionState = groups?.connectionState ?? RealtimeConnectionState.Disconnected;
  const joinGroup = groups?.joinGroup;
  const leaveGroup = groups?.leaveGroup;
  const [isLoading, setIsLoading] = useState(false);
  const [isConnected, setIsConnected] = useState(false);

  const onJoinedRef = useRef(onJoinedGroup);
  const onErrorRef = useRef(onSubscriptionError);

  const lastStateRef = useRef({ isLoading: false, isConnected: false });

  useEffect(() => {
    onJoinedRef.current = onJoinedGroup;
    onErrorRef.current = onSubscriptionError;
  }, [onJoinedGroup, onSubscriptionError]);

  const state = connectionState;

  useEffect(() => {
    const setLoading = (val: boolean) => {
      if (lastStateRef.current.isLoading !== val) {
        lastStateRef.current.isLoading = val;
        setIsLoading(val);
      }
    };
    const setConnected = (val: boolean) => {
      if (lastStateRef.current.isConnected !== val) {
        lastStateRef.current.isConnected = val;
        setIsConnected(val);
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
      setConnected(false);
      return;
    }

    const joined: string[] = [];
    let releasedCount = 0;
    let listenersRemoved = false;
    let disposed = false;

    const limited = new Set<string>();
    const onCapacity = (name: string) => {
      if (!disposed && groupNames.includes(name) && !limited.has(name)) {
        limited.add(name);
        setConnected(false);
        onErrorRef.current?.(new Error(GROUP_CAPACITY_ERROR));
      }
    };
    connection.on('RealtimeGroupCapacity', onCapacity);

    const removeListeners = () => {
      if (listenersRemoved) return;
      listenersRemoved = true;
      connection.off('RealtimeGroupCapacity', onCapacity);
      removeEventListeners?.(connection);
    };

    const leaveJoinedGroups = async () => {
      const groups = joined.slice(releasedCount);
      releasedCount = joined.length;
      await Promise.all(groups.map((currentGroupName) => leaveGroup(currentGroupName)));
    };

    const run = async () => {
      try {
        setLoading(true);
        setupEventListeners(connection);
        for (const currentGroupName of groupNames) {
          try {
            await joinGroup(currentGroupName);
            joined.push(currentGroupName);
          } catch (error) {
            if (!(error instanceof Error) || error.message !== GROUP_CAPACITY_ERROR) throw error;
            onCapacity(currentGroupName);
          }
          if (disposed) break;
        }

        if (disposed) {
          await leaveJoinedGroups();
          removeListeners();
          return;
        }

        setConnected(limited.size === 0);
        setLoading(false);

        if (limited.size === 0) onJoinedRef.current?.(connection);
      } catch (err) {
        if (!disposed) onErrorRef.current?.(err);
        console.error('[Realtime] join failed', err);
        await leaveJoinedGroups();
        removeListeners();
        setConnected(false);
        setLoading(false);
      }
    };

    run();

    return () => {
      disposed = true;
      removeListeners();
      leaveJoinedGroups().catch(console.warn);
    };
  }, [groupName, skip, enabled, state, connection, joinGroup, leaveGroup, setupEventListeners, removeEventListeners]);

  return { isLoading, isConnected };
};
