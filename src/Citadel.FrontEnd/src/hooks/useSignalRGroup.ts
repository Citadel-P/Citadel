import { useEffect, useRef } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { useSignalRContext } from '@/SignalRContext';

/**
 * Options for the `useSignalRGroup` hook.
 */
type UseSignalRGroupOpts = {
  /** The name of the SignalR group to join. */
  groupName?: string;
  /** A function to set up SignalR event listeners on the hub connection. */
  setupEventListeners: (hub: HubConnection) => void;
  /** An optional function to remove SignalR event listeners. This is called on cleanup. */
  removeEventListeners?: (hub: HubConnection) => void;
  /** An optional callback that is executed after successfully joining the group. */
  onJoinedGroup?: (hub: HubConnection) => void;
  /** If true, the hook will not attempt to join the group. */
  skip?: boolean;
};

/**
 * A React hook for managing membership in a SignalR group and handling event listeners.
 * It handles joining the group on mount and leaving the group on unmount,
 * as well as setting up and tearing down event listeners.
 *
 * @param {UseSignalRGroupOpts} opts - The options for the hook.
 */
export const useSignalRGroup = ({
  groupName,
  setupEventListeners,
  removeEventListeners,
  onJoinedGroup,
  skip,
}: UseSignalRGroupOpts) => {
  const { joinGroup, leaveGroup, connection } = useSignalRContext();

  // Use refs to store the latest callbacks to avoid stale closures in useEffect.
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
    if (skip || !groupName || !connection) {
      return;
    }

    let isMounted = true;
    let hasJoinedGroup = false;

    const join = async () => {
      try {
        // The joinGroup function from the context will set up the listeners before joining the group.
        await joinGroup(groupName, (hub) => {
          setupRef.current?.(hub);
        });

        hasJoinedGroup = true;
        if (isMounted) {
          // If the component is still mounted after joining, call the onJoinedGroup callback.
          onJoinedRef.current?.(connection);
        }
      } catch (err) {
        console.error(`Failed to join SignalR group "${groupName}"`, err);
      }
    };

    join();

    return () => {
      isMounted = false;
      (async () => {
        try {
          if (hasJoinedGroup) {
            // If the group was successfully joined, leave the group and remove listeners.
            await leaveGroup(groupName, (hub) => {
              removeRef.current?.(hub);
            });
          } else if (connection) {
            // If not joined, but the connection exists, there's a chance listeners were set up.
            // Attempt to remove them to prevent memory leaks.
            removeRef.current?.(connection);
          }
        } catch (err) {
          console.warn(`Failed to clean up SignalR group "${groupName}"`, err);
        }
      })();
    };
  }, [groupName, skip, connection, joinGroup, leaveGroup]);
};