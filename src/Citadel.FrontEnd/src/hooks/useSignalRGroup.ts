import { useEffect, useRef } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { useSignalRContext } from '@/SignalRContext';

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

  // keep stable refs for callbacks so cleanup is safe
  const setupRef = useRef(setupEventListeners);
  const removeRef = useRef(removeEventListeners);
  const onJoinedRef = useRef(onJoinedGroup);

  useEffect(() => {
    setupRef.current = setupEventListeners;
  }, [setupEventListeners]);
  useEffect(() => {
    removeRef.current = removeEventListeners;
  }, [removeEventListeners]);
  useEffect(() => {
    onJoinedRef.current = onJoinedGroup;
  }, [onJoinedGroup]);

  useEffect(() => {
    if (skip || !groupName || !connection) return;

    let mounted = true;
    let joined = false;

    const start = async () => {
      try {
        // joinGroup will call setupRef.current(conn) before calling JoinGroup
        await joinGroup(groupName, (hub) => {
          // use latest setup function
          setupRef.current?.(hub);
        });
        joined = true;
        if (!mounted) return;
        onJoinedRef.current?.(connection!);
      } catch (err) {
        console.error("Failed to join group", groupName, err);
      }
    };

    start();

    return () => {
      mounted = false;
      (async () => {
        try {
          if (joined) {
            await leaveGroup(groupName, (hub) => {
              // remove handlers
              removeRef.current?.(hub);
            });
          } else {
            // if not joined but connection exists, still remove listeners if they were registered
            if (connection) {
              removeRef.current?.(connection);
            }
          }
        } catch (err) {
          console.warn("cleanup group failed", err);
        }
      })();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [groupName, skip, joinGroup, leaveGroup]);
};