import { useCallback, useRef, useEffect } from 'react';
import { HubConnection } from '@microsoft/signalr';
import {
  ContainerView,
  DockerNetworkResultView,
  DockerVolumeResultView,
  ImageView,
  SwarmConfigsView,
  SwarmNetworksView,
  SwarmNodesView,
  SwarmSecretsView,
  SwarmServicesView,
  SwarmTasksView,
} from '@/api/generated/api.types';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';

export const useDockerDaemonGroup = (platformId?: string, listeners?: DockerDaemonListeners) => {
  const listenersRef = useRef(listeners);
  useEffect(() => {
    listenersRef.current = listeners;
  }, [listeners]);

  const handleContainerEventReceived = useCallback((container: ContainerView, eventType: string) => {
    listenersRef.current?.onContainerEvent?.({ container, eventType });
  }, []);

  const handleImageEventReceived = useCallback((image: ImageView, eventType: string) => {
    listenersRef.current?.onImageEvent?.({ image, eventType });
  }, []);

  const handleVolumeEventReceived = useCallback(
    (volume: DockerVolumeResultView, eventType: string, actorId: string) => {
      listenersRef.current?.onVolumeEvent?.({ volume, eventType, actorId });
    },
    [],
  );

  const handleNetworkEventReceived = useCallback(
    (network: DockerNetworkResultView, eventType: string, actorId: string) => {
      listenersRef.current?.onNetworkEvent?.({ network, eventType, actorId });
    },
    [],
  );

  const handleSwarmInventoryUpdated = useCallback((inventory: SwarmInventoryUpdate) => {
    listenersRef.current?.onSwarmInventoryUpdated?.(inventory);
  }, []);

  const setupEventListeners = useCallback(
    (hub: HubConnection) => {
      hub.on('ImageEventReceived', handleImageEventReceived);
      hub.on('VolumeEventReceived', handleVolumeEventReceived);
      hub.on('NetworkEventReceived', handleNetworkEventReceived);
      hub.on('ContainerEventReceived', handleContainerEventReceived);
      hub.on('SwarmInventoryUpdated', handleSwarmInventoryUpdated);
    },
    [
      handleContainerEventReceived,
      handleImageEventReceived,
      handleVolumeEventReceived,
      handleNetworkEventReceived,
      handleSwarmInventoryUpdated,
    ],
  );

  const removeEventListeners = useCallback(
    (hub: HubConnection) => {
      hub.off('ImageEventReceived', handleImageEventReceived);
      hub.off('VolumeEventReceived', handleVolumeEventReceived);
      hub.off('NetworkEventReceived', handleNetworkEventReceived);
      hub.off('ContainerEventReceived', handleContainerEventReceived);
      hub.off('SwarmInventoryUpdated', handleSwarmInventoryUpdated);
    },
    [
      handleContainerEventReceived,
      handleImageEventReceived,
      handleVolumeEventReceived,
      handleNetworkEventReceived,
      handleSwarmInventoryUpdated,
    ],
  );

  useSignalRGroup({
    groupName: `docker-daemon:${platformId}`,
    setupEventListeners,
    removeEventListeners,
    skip: !platformId,
  });
};

export type BaseEvent = {
  eventType: string;
};

export interface ContainerEvent extends BaseEvent {
  container: ContainerView;
}

export interface ImageEvent extends BaseEvent {
  image: ImageView;
}

export interface VolumeEvent extends BaseEvent {
  volume: DockerVolumeResultView;
  actorId: string;
}

export interface NetworkEvent extends BaseEvent {
  network: DockerNetworkResultView;
  actorId: string;
}

export type DockerDaemonListeners = {
  onContainerEvent?: (event: ContainerEvent) => void;
  onImageEvent?: (event: ImageEvent) => void;
  onVolumeEvent?: (event: VolumeEvent) => void;
  onNetworkEvent?: (event: NetworkEvent) => void;
  onSwarmInventoryUpdated?: (inventory: SwarmInventoryUpdate) => void;
};

export type SwarmInventoryUpdate = {
  platformId: string;
  nodes: SwarmNodesView;
  services: SwarmServicesView;
  tasks: SwarmTasksView;
  networks: SwarmNetworksView;
  secrets: SwarmSecretsView;
  configs: SwarmConfigsView;
};
