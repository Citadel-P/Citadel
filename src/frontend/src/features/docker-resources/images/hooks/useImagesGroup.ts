import { useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import type { ImagesResponse } from '@/api/generated/api.types';
import type { ResourceResponse } from '@/api/types';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';

export const useImagesGroup = (platformId?: string) => {
  const queryClient = useQueryClient();
  const { data, isLoading, error, refetch, isFetching } = useRead('listImages', { platformId });
  const onImagesUpdated = useCallback(
    (snapshot: ImagesResponse & { platformId: string }) => {
      if (!platformId || snapshot.platformId !== platformId) return;
      const queryKey = ['listImages', { platformId }] as const;
      // As with other resource subscriptions, a slow HTTP response must not
      // overwrite a newer, actor-authorized realtime snapshot.
      void queryClient.cancelQueries({ queryKey, exact: true });
      queryClient.setQueryData<ResourceResponse<'listImages'>>(
        queryKey,
        (current) =>
          ({
            ...current,
            data: { images: snapshot.images, capabilities: snapshot.capabilities },
          }) as ResourceResponse<'listImages'>,
      );
    },
    [platformId, queryClient],
  );
  const setupEventListeners = useCallback(
    (connection: RealtimeConnection) => {
      connection.on('ImagesInfoUpdated', onImagesUpdated);
    },
    [onImagesUpdated],
  );
  const removeEventListeners = useCallback(
    (connection: RealtimeConnection) => {
      connection.off('ImagesInfoUpdated', onImagesUpdated);
    },
    [onImagesUpdated],
  );
  useRealtimeGroup({ groupName: `images:${platformId}`, setupEventListeners, removeEventListeners, skip: !platformId });
  return { error, refetch, isFetching, imagesInfo: data?.data, isLoading, capabilities: data?.data.capabilities };
};
