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

  const onJoinedRef = useRef(onJoinedGroup);
  const activeSetupRef = useRef(setupEventListeners);
  const effectGenerationRef = useRef(0);

  const lastStateRef = useRef({ isLoading: false, isConnected: false });

  useEffect(() => {
    onJoinedRef.current = onJoinedGroup;
  }, [onJoinedGroup]);

  useEffect(() => {
    activeSetupRef.current = setupEventListeners;
  }, [setupEventListeners]);

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

    const generation = ++effectGenerationRef.current;
    let joined = false;
    let disposed = false;

    const run = async () => {
      try {
        setLoading(true);
        await joinGroup(groupName, setupEventListeners);

        if (disposed) {
          const replayedWithSameListeners =
            generation !== effectGenerationRef.current &&
            activeSetupRef.current === setupEventListeners;

          await leaveGroup(
            groupName,
            replayedWithSameListeners ? undefined : removeEventListeners,
          );
          return;
        }

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
      disposed = true;
      if (joined) {
        leaveGroup(groupName, removeEventListeners).catch(console.warn);
      }
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
