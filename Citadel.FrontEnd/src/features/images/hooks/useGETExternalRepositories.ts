import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useQuery } from '@tanstack/react-query';

export const useGETExternalRepositories = (registryName: string | undefined) => {
  const apiClient = useContextSelector(ApiClientContext, (v) => v?.apiClient);
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['externalRepositories', registryName],
    queryFn: ({ signal }) => apiClient?.api.imagesGetExternalRepositories(registryName!, { signal }),
    enabled: !!registryName,
  });

  return { data, error, isLoading, isSuccess };
};
