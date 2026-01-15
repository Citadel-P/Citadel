import { useState, useCallback } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { ContainerView, DockerNetworkResult, DockerVolumeResult, ImageView } from '@/api/generated/api.types';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';

export const useDockerDaemonGroup = (platformId?: string) => {
  const [imageEvent, setImageEvent] = useState<ImageEvent | undefined>();
  const [volumeEvent, setVolumeEvent] = useState<VolumeEvent | undefined>();
  const [networkEvent, setNetworkEvent] = useState<NetworkEvent | undefined>();
  const [containerEvent, setContainerEvent] = useState<ContainerEvent | undefined>();

  const handleContainerEventReceived = useCallback((container: ContainerView, eventType: string) => {
    setContainerEvent({ container, eventType });
  }, []);

  const handleImageEventReceived = useCallback((image: ImageView, eventType: string) => {
    setImageEvent({ image, eventType });
  }, []);

  const handleVolumeEventReceived = useCallback((volume: DockerVolumeResult, eventType: string, actorId: string) => {
    setVolumeEvent({ volume, eventType, actorId });
  }, []);

  const handleNetworkEventReceived = useCallback((network: DockerNetworkResult, eventType: string, actorId: string) => {
    setNetworkEvent({ network, eventType, actorId });
  }, []);

  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.on('ImageEventReceived', handleImageEventReceived);
      hubConnection.on('VolumeEventReceived', handleVolumeEventReceived);
      hubConnection.on('NetworkEventReceived', handleNetworkEventReceived);
      hubConnection.on('ContainerEventReceived', handleContainerEventReceived);
    },
    [handleContainerEventReceived, handleImageEventReceived, handleVolumeEventReceived, handleNetworkEventReceived],
  );

  const removeEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.off('ImageEventReceived', handleImageEventReceived);
      hubConnection.off('VolumeEventReceived', handleVolumeEventReceived);
      hubConnection.off('NetworkEventReceived', handleNetworkEventReceived);
      hubConnection.off('ContainerEventReceived', handleContainerEventReceived);
    },
    [handleContainerEventReceived, handleImageEventReceived, handleVolumeEventReceived, handleNetworkEventReceived],
  );

  useSignalRGroup({
    groupName: `docker-daemon:${platformId}`,
    setupEventListeners,
    removeEventListeners,
    skip: !platformId,
  });

  return { containerEvent, imageEvent, volumeEvent, networkEvent };
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

interface VolumeEvent extends BaseEvent {
  volume: DockerVolumeResult;
  actorId: string;
}

interface NetworkEvent extends BaseEvent {
  network: DockerNetworkResult;
  actorId: string;
}
