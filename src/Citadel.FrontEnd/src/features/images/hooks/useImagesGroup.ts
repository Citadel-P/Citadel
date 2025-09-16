import { useEffect, useState, useCallback } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { ImagesView } from '@/api/_generated';
import { useDockerDaemonGroup } from '@/features/platforms/hooks/useDockerDaemonGroup';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';

export const useImagesGroup = (platformId?: string) => {
  const [isLoading, setIsLoading] = useState(false);
  const [imagesInfo, setimagesInfo] = useState<ImagesView | undefined>();
  const { containerEvent } = useDockerDaemonGroup(platformId);

  const handleimagesInfoUpdated = useCallback((images: ImagesView) => {
    setimagesInfo(images);
  }, []);

  //   useEffect(() => {
  //     setimagesInfo((currentInfo) => {
  //       if (!currentInfo) {
  //         return currentInfo;
  //       }

  //       const updatedImages = [...(currentInfo.images ?? [])];
  //       const existingIndex = updatedImages.findIndex((c) => c.containerId === containerEvent?.container.containerId);

  //       switch (containerEvent?.eventType) {
  //         case 'create':
  //           if (existingIndex === -1) {
  //             return { ...currentInfo, images: [containerEvent.container, ...updatedImages] };
  //           }
  //           if (JSON.stringify(updatedImages[existingIndex]) !== JSON.stringify(containerEvent.container)) {
  //             updatedImages[existingIndex] = containerEvent.container;
  //             return { ...currentInfo, images: updatedImages };
  //           }
  //           break;

  //         case 'destroy':
  //           if (existingIndex !== -1) {
  //             updatedImages.splice(existingIndex, 1);
  //             return { ...currentInfo, images: updatedImages };
  //           }
  //           break;

  //         default:
  //           if (
  //             existingIndex !== -1 &&
  //             JSON.stringify(updatedImages[existingIndex]) !== JSON.stringify(containerEvent?.container)
  //           ) {
  //             if (containerEvent?.container) {
  //               updatedImages[existingIndex] = containerEvent?.container;
  //             }
  //             return { ...currentInfo, images: updatedImages };
  //           }
  //           break;
  //       }

  //       return currentInfo;
  //     });
  //   }, [containerEvent]);

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
      hubConnection.on('ImagesInfoUpdated', handleimagesInfoUpdated);
    },
    [handleimagesInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.off('ImagesInfoUpdated', handleimagesInfoUpdated);
    },
    [handleimagesInfoUpdated],
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
