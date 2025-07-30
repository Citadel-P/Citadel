import { useApiClientContext } from '@/api/ApiClientContext';
import { useQuery } from '@tanstack/react-query';

export const useGETPackageVersions = (registryName: string | undefined, packageName: string | undefined) => {
  const { apiClient } = useApiClientContext();
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['GhcrPackageVersions', registryName, packageName],
    queryFn: ({ signal }) => apiClient?.api.imagesGetGhcrPackageVersions(registryName!, packageName!, { signal }),
    enabled: !!registryName && !!packageName,
  });

  return { data, error, isLoading, isSuccess };
};
