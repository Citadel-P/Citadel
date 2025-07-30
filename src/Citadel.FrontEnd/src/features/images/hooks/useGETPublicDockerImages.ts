import { useApiClientContext } from '@/api/ApiClientContext';
import { useQuery } from '@tanstack/react-query';

export const useGETPublicDockerImages = (searchValue?: string | undefined) => {
  const { apiClient } = useApiClientContext();
  const { data, error, isLoading, isSuccess, refetch } = useQuery({
    queryKey: ['publicDockerImages', searchValue],
    queryFn: ({ signal }) => apiClient?.api.imagesGetDockerHubPublicImages({ imageName: searchValue }, { signal }),
  });

  return { data, error, isLoading, isSuccess, refetch };
};
