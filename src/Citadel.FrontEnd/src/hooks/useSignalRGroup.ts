import { useEffect, useRef, useState } from 'react';
import { HubConnection, HubConnectionState } from '@microsoft/signalr';
import { useSignalRContext } from '@/lib/context/signalr-context';

type UseSignalRGroupOpts = {
  groupName?: string | readonly string[];
  setupEventListeners: (hub: HubConnection) => void;
  removeEventListeners?: (hub: HubConnection) => void;
  onJoinedGroup?: (hub: HubConnection) => void;
  skip?: boolean;
  enabled?: boolean;
};

export const useSignalRGroup = ({
  groupName,
  setupEventListeners,
  removeEventListeners,
  onJoinedGroup,
  skip,
  enabled = true,
}: UseSignalRGroupOpts) => {
  const { joinGroup, leaveGroup, connection, connectionState } = useSignalRContext();
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

    const groupNames = typeof groupName === 'string' ? [groupName] : groupName ?? [];
    if (skip || !enabled || !groupNames.length || !connection || state !== HubConnectionState.Connected) {
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
        console.error('[SignalR] join failed', err);
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
  ]);

  return { isLoading, isConnected };
};
