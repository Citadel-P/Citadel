import { useEffect, useRef, useState } from 'react';
import { RealtimeConnection, RealtimeConnectionState } from '@/lib/realtime-connection';
import { useRealtimeContext } from '@/lib/context/realtime-context';

type UseRealtimeGroupOpts = {
  groupName?: string | readonly string[];
  setupEventListeners: (hub: RealtimeConnection) => void;
  removeEventListeners?: (hub: RealtimeConnection) => void;
  onJoinedGroup?: (hub: RealtimeConnection) => void;
  skip?: boolean;
  enabled?: boolean;
};

export const useRealtimeGroup = ({
  groupName,
  setupEventListeners,
  removeEventListeners,
  onJoinedGroup,
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

  const lastStateRef = useRef({ isLoading: false, isConnected: false });

  useEffect(() => {
    onJoinedRef.current = onJoinedGroup;
  }, [onJoinedGroup]);

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

    const removeListeners = () => {
      if (listenersRemoved) return;
      listenersRemoved = true;
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
          await joinGroup(currentGroupName);
          joined.push(currentGroupName);
          if (disposed) break;
        }

        if (disposed) {
          await leaveJoinedGroups();
          removeListeners();
          return;
        }

        setConnected(true);
        setLoading(false);

        onJoinedRef.current?.(connection);
      } catch (err) {
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
