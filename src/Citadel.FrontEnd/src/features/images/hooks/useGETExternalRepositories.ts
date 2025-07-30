import { useApiClientContext } from '@/api/ApiClientContext';
import { useQuery } from '@tanstack/react-query';

export const useGETExternalRepositories = (registryName: string | undefined) => {
  const { apiClient } = useApiClientContext();
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['externalRepositories', registryName],
    queryFn: ({ signal }) => apiClient?.api.imagesGetExternalRepositories(registryName!, { signal }),
    enabled: !!registryName,
  });

  return { data, error, isLoading, isSuccess };
};
