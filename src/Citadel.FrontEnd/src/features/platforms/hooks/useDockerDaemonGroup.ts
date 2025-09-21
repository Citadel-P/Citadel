import { useState, useCallback } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { ContainerView, ImageView } from '@/api/_generated';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';

export const useDockerDaemonGroup = (platformId?: string) => {
  const [containerEvent, setContainerEvent] = useState<ContainerEvent | undefined>();
  const [imageEvent, setImageEvent] = useState<ImageEvent | undefined>();

  const handleContainerEventReceived = useCallback((container: ContainerView, eventType: string) => {
    setContainerEvent({ container, eventType });
  }, []);

  const handleImageEventReceived = useCallback((image: ImageView, eventType: string) => {
    setImageEvent({ image, eventType });
  }, []);

  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.on('ContainerEventReceived', handleContainerEventReceived);
      hubConnection.on('ImageEventReceived', handleImageEventReceived);
    },
    [handleContainerEventReceived, handleImageEventReceived],
  );

  const removeEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.off('ContainerEventReceived', handleContainerEventReceived);
      hubConnection.off('ImageEventReceived', handleImageEventReceived);
    },
    [handleContainerEventReceived, handleImageEventReceived],
  );

  useSignalRGroup({
    groupName: `docker-daemon:${platformId}`,
    setupEventListeners,
    removeEventListeners,
    skip: !platformId,
  });

  return { containerEvent, imageEvent };
};
type BaseEvent = {
  eventType: string;
};

interface ContainerEvent extends BaseEvent {
  container: ContainerView;
}

interface ImageEvent extends BaseEvent {
  image: ImageView;
}
