import { useEffect, useState, useCallback } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { ImagesView, ImageView } from '@/api/generated/api.types';
import { useDockerDaemonGroup } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';

export const useImagesGroup = (platformId?: string) => {
  const [isLoading, setIsLoading] = useState(false);
  const [imagesInfo, setimagesInfo] = useState<ImagesView | undefined>();
  const { imageEvent } = useDockerDaemonGroup(platformId);

  const handleImagesInfoUpdated = useCallback((images: ImagesView) => {
    setimagesInfo(images);
  }, []);

  const handleImageInfoUpdated = useCallback((image: ImageView) => {
    setimagesInfo((currentInfo) => {
      if (!currentInfo) {
        return currentInfo;
      }

      const updatedImages = [...(currentInfo.images ?? [])];
      const existingIndex = updatedImages.findIndex((c) => c.imageId === image.imageId);

      if (existingIndex === -1) {
        return { ...currentInfo, images: [image, ...updatedImages] };
      }
      if (JSON.stringify(updatedImages[existingIndex]) !== JSON.stringify(image)) {
        updatedImages[existingIndex] = image;
        return { ...currentInfo, images: updatedImages };
      }

      return currentInfo;
    });
  }, []);

  useEffect(() => {
    setimagesInfo((currentInfo) => {
      if (!currentInfo) {
        return currentInfo;
      }

      const updatedImages = [...(currentInfo.images ?? [])];
      const existingIndex = updatedImages.findIndex((c) => c.imageId === imageEvent?.image.imageId);

      switch (imageEvent?.eventType) {
        case 'delete':
          if (existingIndex !== -1) {
            updatedImages.splice(existingIndex, 1);
            return { ...currentInfo, images: updatedImages };
          }
          break;

        default:
          break;
      }

      return currentInfo;
    });
  }, [imageEvent]);

  const getImagesList = useCallback(
    async (hubConnection: HubConnection) => {
      try {
        setIsLoading(true);
        const images = await hubConnection.invoke<ImagesView>('GetImages', platformId);
        if (images) {
          setimagesInfo(images);
        }
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

  const onJoinedGroup = useCallback(
    (hubConnection: HubConnection) => {
      if (!hubConnection) return;
      getImagesList(hubConnection);
      hubConnection.onreconnected(() => {
        getImagesList(hubConnection);
      });
    },
    [getImagesList],
  );

  useSignalRGroup({
    groupName: `images:${platformId}`,
    setupEventListeners,
    removeEventListeners,
    onJoinedGroup,
    skip: !platformId,
  });

  return { imagesInfo, isLoading };
};
