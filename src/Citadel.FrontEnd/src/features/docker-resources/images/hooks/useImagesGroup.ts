import { useState, useCallback, useMemo } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { ImagesView, ImageView } from '@/api/generated/api.types';
import {
  useDockerDaemonGroup,
  ImageEvent,
  SwarmNodeLocalResourcesUpdate,
} from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { useRead } from '@/lib/hooks';

export const useImagesGroup = (platformId?: string) => {
  const { data, isLoading } = useRead('listImages', { platformId });
  const [realtimeImagesInfo, setRealtimeImagesInfo] = useState<ImagesView>();
  const [nodeSnapshot, setNodeSnapshot] = useState<SwarmNodeLocalResourcesUpdate>();
  const capabilities = data?.data.capabilities;

  const imagesInfo = useMemo<ImagesView | undefined>(() => {
    const source = realtimeImagesInfo ?? data?.data;
    return source && nodeSnapshot?.platformId === platformId
      ? { ...source, images: nodeSnapshot.images }
      : source;
  }, [data, nodeSnapshot, platformId, realtimeImagesInfo]);

  const onImageEvent = useCallback(
    (event: ImageEvent) => {
      setRealtimeImagesInfo((currentInfo) => {
        const source = currentInfo ?? data?.data;

        if (!source?.images) {
          return source;
        }

        const { image, eventType } = event;

        const existingIndex = source.images.findIndex((i) => i.dockerImageId === image.dockerImageId);

        if (eventType === 'delete' || eventType === 'untag') {
          if (existingIndex !== -1) {
            return {
              ...source,
              images: source.images.filter((i) => i.dockerImageId !== image.dockerImageId),
            };
          }

          return source;
        }

        if (existingIndex !== -1) {
          const updatedImages = [...source.images];

          updatedImages[existingIndex] = {
            ...updatedImages[existingIndex],
            controlState: image.controlState,
          };

          return {
            ...source,
            images: updatedImages,
          };
        }

        return source;
      });
    },
    [data],
  );

  const onSwarmNodeLocalResourcesUpdated = useCallback(
    (snapshot: SwarmNodeLocalResourcesUpdate) => {
      if (snapshot.platformId !== platformId) return;
      setNodeSnapshot(snapshot);
      setRealtimeImagesInfo((currentInfo) => {
        const source = currentInfo ?? data?.data;
        return source ? { ...source, images: snapshot.images } : source;
      });
    },
    [data, platformId],
  );

  useDockerDaemonGroup(platformId, { onImageEvent, onSwarmNodeLocalResourcesUpdated });

  const handleImageInfoUpdated = useCallback(
    (image: ImageView) => {
      setRealtimeImagesInfo((currentInfo) => {
        const source = currentInfo ?? data?.data;

        if (!source) {
          return source;
        }

        const updatedImages = [...(source.images ?? [])];

        const existingIndex = updatedImages.findIndex((i) => i.dockerImageId === image.dockerImageId);

        if (existingIndex === -1) {
          return {
            ...source,
            images: [image, ...updatedImages],
          };
        }

        updatedImages[existingIndex] = image;

        return {
          ...source,
          images: updatedImages,
        };
      });
    },
    [data],
  );

  const handleImagesInfoUpdated = useCallback((images: ImagesView) => {
    setRealtimeImagesInfo(images);
  }, []);

  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.on('ImageInfoUpdated', handleImageInfoUpdated);

      hubConnection.on('ImagesInfoUpdated', handleImagesInfoUpdated);
    },
    [handleImagesInfoUpdated, handleImageInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.off('ImagesInfoUpdated', handleImagesInfoUpdated);

      hubConnection.off('ImageInfoUpdated', handleImageInfoUpdated);
    },
    [handleImagesInfoUpdated, handleImageInfoUpdated],
  );

  useSignalRGroup({
    groupName: `images:${platformId}`,
    setupEventListeners,
    removeEventListeners,
    skip: !platformId,
  });

  return {
    imagesInfo,
    isLoading,
    capabilities,
  };
};
