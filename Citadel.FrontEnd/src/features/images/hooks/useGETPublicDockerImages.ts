import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useQuery } from '@tanstack/react-query';

export const useGETPublicDockerImages = (imageName: string | undefined) => {
  const apiClient = useContextSelector(ApiClientContext, (v) => v?.apiClient);
  const { data, error, isLoading, isSuccess, refetch } = useQuery({
    queryKey: ['getPublicDockerImages'],
    queryFn: ({ signal }) => apiClient?.api.imagesGetDockerHubPublicImages({ imageName }, { signal }),
  });

  return { data, error, isLoading, isSuccess, refetch };
};
