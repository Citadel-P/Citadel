import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useQuery } from '@tanstack/react-query';

export const useGETExternalImages = (registryName: string | undefined) => {
  const apiClient = useContextSelector(ApiClientContext, (v) => v?.apiClient);
  console.log(apiClient?.api, registryName)
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['externalImages', registryName],
    queryFn: ({ signal }) => apiClient?.api.imagesGetExternalImages(registryName!, { signal }),
    enabled: !!registryName,
  });

  return { data, error, isLoading, isSuccess };
};
