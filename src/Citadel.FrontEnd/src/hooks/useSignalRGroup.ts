import { useEffect, useRef, useState } from 'react';
import { HubConnection, HubConnectionState } from '@microsoft/signalr';
import { useSignalRContext } from '@/lib/context/signalr-context';

type UseSignalRGroupOpts = {
  groupName?: string;
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

  const setupRef = useRef(setupEventListeners);
  const onJoinedRef = useRef(onJoinedGroup);

  const lastStateRef = useRef({ isLoading: false, isConnected: false });

  useEffect(() => {
    setupRef.current = setupEventListeners;
  }, [setupEventListeners]);

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

    if (skip || !enabled || !groupName || !connection || state !== HubConnectionState.Connected) {
      setConnected(false);
      return;
    }

    let joined = false;

    const run = async () => {
      try {
        setLoading(true);
        await joinGroup(groupName, (hub) => setupRef.current(hub));
        joined = true;
        setConnected(true);
        setLoading(false);

        onJoinedRef.current?.(connection);
      } catch (err) {
        console.error('[SignalR] join failed', err);
        setConnected(false);
        setLoading(false);
      }
    };

    run();

    return () => {
      if (joined) {
        leaveGroup(groupName, (hub) => removeEventListeners?.(hub)).catch(console.warn);
      }
    };
  }, [groupName, skip, enabled, state, connection, joinGroup, leaveGroup, removeEventListeners]);

  return { isLoading, isConnected };
};
