import { useCallback, useRef, useEffect } from 'react';
import { RealtimeConnection } from '@/lib/realtime-connection';
import {
  ContainerView,
  NetworkView,
  VolumeView,
  ImageView,
  SwarmItemsResponseSwarmConfigView,
  SwarmItemsResponseSwarmNetworkView,
  SwarmItemsResponseSwarmNodeView,
  SwarmItemsResponseSwarmSecretView,
  SwarmItemsResponseSwarmServiceView,
  SwarmItemsResponseSwarmTaskView,
} from '@/api/generated/api.types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import type { ContainerStatePatch } from '@/features/docker-resources/containers/hooks/container-order';

export const useDockerDaemonGroup = (platformId?: string, listeners?: DockerDaemonListeners) => {
  const listenersRef = useRef(listeners);
  useEffect(() => {
    listenersRef.current = listeners;
  }, [listeners]);

  const handleContainerEventReceived = useCallback((container: ContainerView, eventType: string) => {
    listenersRef.current?.onContainerEvent?.({ container, eventType });
  }, []);

  const handleContainerStateChanged = useCallback((patches: ContainerStatePatch[]) => {
    listenersRef.current?.onContainerStateChange?.(patches);
  }, []);

  const handleImageEventReceived = useCallback((image: ImageView, eventType: string) => {
    listenersRef.current?.onImageEvent?.({ image, eventType });
  }, []);

  const handleVolumeEventReceived = useCallback(
    (volume: VolumeView, eventType: string, actorId: string) => {
      listenersRef.current?.onVolumeEvent?.({ volume, eventType, actorId });
    },
    [],
  );

  const handleNetworkEventReceived = useCallback(
    (network: NetworkView, eventType: string, actorId: string) => {
      listenersRef.current?.onNetworkEvent?.({ network, eventType, actorId });
    },
    [],
  );

  const handleSwarmInventoryUpdated = useCallback((inventory: SwarmInventoryUpdate) => {
    listenersRef.current?.onSwarmInventoryUpdated?.(inventory);
  }, []);

  const handleSwarmNodeAgentCoverageChanged = useCallback((updatedPlatformId: string) => {
    listenersRef.current?.onSwarmNodeAgentCoverageChanged?.(updatedPlatformId);
  }, []);

  const handleSwarmNodeLocalResourcesUpdated = useCallback((snapshot: SwarmNodeLocalResourcesUpdate) => {
    listenersRef.current?.onSwarmNodeLocalResourcesUpdated?.(snapshot);
  }, []);

  const setupEventListeners = useCallback(
    (hub: RealtimeConnection) => {
      hub.on('ImageEventReceived', handleImageEventReceived);
      hub.on('VolumeEventReceived', handleVolumeEventReceived);
      hub.on('NetworkEventReceived', handleNetworkEventReceived);
      hub.on('ContainerEventReceived', handleContainerEventReceived);
      hub.on('ContainerStateChanged', handleContainerStateChanged);
      hub.on('SwarmInventoryUpdated', handleSwarmInventoryUpdated);
      hub.on('SwarmNodeAgentCoverageChanged', handleSwarmNodeAgentCoverageChanged);
      hub.on('SwarmNodeLocalResourcesUpdated', handleSwarmNodeLocalResourcesUpdated);
    },
    [
      handleContainerEventReceived,
      handleContainerStateChanged,
      handleImageEventReceived,
      handleVolumeEventReceived,
      handleNetworkEventReceived,
      handleSwarmNodeAgentCoverageChanged,
      handleSwarmNodeLocalResourcesUpdated,
      handleSwarmInventoryUpdated,
    ],
  );

  const removeEventListeners = useCallback(
    (hub: RealtimeConnection) => {
      hub.off('ImageEventReceived', handleImageEventReceived);
      hub.off('VolumeEventReceived', handleVolumeEventReceived);
      hub.off('NetworkEventReceived', handleNetworkEventReceived);
      hub.off('ContainerEventReceived', handleContainerEventReceived);
      hub.off('ContainerStateChanged', handleContainerStateChanged);
      hub.off('SwarmInventoryUpdated', handleSwarmInventoryUpdated);
      hub.off('SwarmNodeAgentCoverageChanged', handleSwarmNodeAgentCoverageChanged);
      hub.off('SwarmNodeLocalResourcesUpdated', handleSwarmNodeLocalResourcesUpdated);
    },
    [
      handleContainerEventReceived,
      handleContainerStateChanged,
      handleImageEventReceived,
      handleVolumeEventReceived,
      handleNetworkEventReceived,
      handleSwarmNodeAgentCoverageChanged,
      handleSwarmNodeLocalResourcesUpdated,
      handleSwarmInventoryUpdated,
    ],
  );

  useRealtimeGroup({
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
  volume: VolumeView;
  actorId: string;
}

export interface NetworkEvent extends BaseEvent {
  network: NetworkView;
  actorId: string;
}

export type DockerDaemonListeners = {
  onContainerEvent?: (event: ContainerEvent) => void;
  onContainerStateChange?: (patches: ContainerStatePatch[]) => void;
  onImageEvent?: (event: ImageEvent) => void;
  onVolumeEvent?: (event: VolumeEvent) => void;
  onNetworkEvent?: (event: NetworkEvent) => void;
  onSwarmInventoryUpdated?: (inventory: SwarmInventoryUpdate) => void;
  onSwarmNodeAgentCoverageChanged?: (platformId: string) => void;
  onSwarmNodeLocalResourcesUpdated?: (snapshot: SwarmNodeLocalResourcesUpdate) => void;
};

export type SwarmInventoryUpdate = {
  platformId: string;
  nodes: SwarmItemsResponseSwarmNodeView;
  services: SwarmItemsResponseSwarmServiceView;
  tasks: SwarmItemsResponseSwarmTaskView;
  networks: SwarmItemsResponseSwarmNetworkView;
  secrets: SwarmItemsResponseSwarmSecretView;
  configs: SwarmItemsResponseSwarmConfigView;
};

export type SwarmNodeLocalResourcesUpdate = {
  platformId: string;
  images?: ImageView[];
  volumes?: VolumeView[];
  networks?: NetworkView[];
  nodeOnly?: boolean;
  volumeCount?: number;
};
