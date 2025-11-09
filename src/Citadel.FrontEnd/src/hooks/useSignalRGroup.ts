import { useEffect, useRef, useState } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { useSignalRContext } from '@/lib/context/signalr-context';

type UseSignalRGroupOpts = {
  groupName?: string;
  setupEventListeners: (hub: HubConnection) => void;
  removeEventListeners?: (hub: HubConnection) => void;
  onJoinedGroup?: (hub: HubConnection) => void;
  skip?: boolean;
};

export const useSignalRGroup = ({
  groupName,
  setupEventListeners,
  removeEventListeners,
  onJoinedGroup,
  skip,
}: UseSignalRGroupOpts) => {
  const { joinGroup, leaveGroup, connection } = useSignalRContext();

  const setupRef = useRef(setupEventListeners);
  const removeRef = useRef(removeEventListeners);
  const onJoinedRef = useRef(onJoinedGroup);

  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => void (setupRef.current = setupEventListeners), [setupEventListeners]);
  useEffect(() => void (removeRef.current = removeEventListeners), [removeEventListeners]);
  useEffect(() => void (onJoinedRef.current = onJoinedGroup), [onJoinedGroup]);

  useEffect(() => {
    if (skip || !groupName || !connection) {
      setIsLoading(false);
      return;
    }

    let cancelled = false;
    let joined = false;

    const join = async () => {
      setIsLoading(true);
      try {
        await joinGroup(groupName, (hub) => setupRef.current?.(hub));
        joined = true;
        if (!cancelled) {
          onJoinedRef.current?.(connection);
          setIsLoading(false);
        }
      } catch (err) {
        if (!cancelled) {
          console.error(`Failed to join SignalR group "${groupName}"`, err);
          setIsLoading(false);
        }
      }
    };

    join();

    return () => {
      cancelled = true;

      if (joined) {
        leaveGroup(groupName, (hub) => removeRef.current?.(hub)).catch((err) =>
          console.warn(`Failed to leave SignalR group "${groupName}"`, err),
        );
      } else if (connection) {
        // In case listeners were partially attached
        removeRef.current?.(connection);
      }
    };
  }, [groupName, skip, connection, joinGroup, leaveGroup]);

  return { isLoading };
};
