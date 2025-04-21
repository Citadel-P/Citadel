import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useQuery } from '@tanstack/react-query';

export const useGETPublicDockerImages = (searchValue?: string | undefined) => {
  const apiClient = useContextSelector(ApiClientContext, (v) => v?.apiClient);
  const { data, error, isLoading, isSuccess, refetch } = useQuery({
    queryKey: ['publicDockerImages', searchValue],
    queryFn: ({ signal }) => apiClient?.api.imagesGetDockerHubPublicImages({ imageName: searchValue }, { signal }),
  });

  return { data, error, isLoading, isSuccess, refetch };
};
