import { useState, useCallback } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { ImagesView, ImageView } from '@/api/generated/api.types';
import { useDockerDaemonGroup, ImageEvent } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';

export const useImagesGroup = (platformId?: string) => {
  const [isLoading, setIsLoading] = useState(false);
  const [imagesInfo, setimagesInfo] = useState<ImagesView | undefined>();

  const onImageEvent = useCallback((event: ImageEvent) => {
    setimagesInfo((currentInfo) => {
      if (!currentInfo?.images) return currentInfo;

      const { image, eventType } = event;
      const existingIndex = currentInfo.images.findIndex((i) => i.dockerImageId === image.dockerImageId);

      if (eventType === 'delete' || eventType === 'untag') {
        if (existingIndex !== -1) {
          return {
            ...currentInfo,
            images: currentInfo.images.filter((i) => i.dockerImageId !== image.dockerImageId),
          };
        }
      }

      if (existingIndex !== -1) {
        const updatedImages = [...currentInfo.images];
        updatedImages[existingIndex] = {
          ...updatedImages[existingIndex],
          controlState: image.controlState,
        };
        return { ...currentInfo, images: updatedImages };
      }

      return currentInfo;
    });
  }, []);

  useDockerDaemonGroup(platformId, { onImageEvent });

  const handleImageInfoUpdated = useCallback((image: ImageView) => {
    setimagesInfo((currentInfo) => {
      if (!currentInfo) return currentInfo;

      const updatedImages = [...(currentInfo.images ?? [])];
      const existingIndex = updatedImages.findIndex((i) => i.dockerImageId === image.dockerImageId);

      if (existingIndex === -1) {
        return { ...currentInfo, images: [image, ...updatedImages] };
      }

      updatedImages[existingIndex] = image;
      return { ...currentInfo, images: updatedImages };
    });
  }, []);

  const handleImagesInfoUpdated = useCallback((images: ImagesView) => {
    setimagesInfo(images);
  }, []);

  const getImagesList = useCallback(
    async (hubConnection: HubConnection) => {
      if (!platformId) return;
      try {
        setIsLoading(true);
        const images = await hubConnection.invoke<ImagesView>('GetImages', platformId);
        if (images) setimagesInfo(images);
      } catch (err) {
        console.error('Failed to fetch images', err);
      } finally {
        setIsLoading(false);
      }
    },
    [platformId],
  );

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
    // Note: removed manual onreconnected leak.
    // useSignalRGroup should call onJoinedGroup again automatically on reconnect.
    onJoinedGroup: getImagesList,
    skip: !platformId,
  });

  return { imagesInfo, isLoading };
};
