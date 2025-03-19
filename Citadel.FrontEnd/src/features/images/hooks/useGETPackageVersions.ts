import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useQuery } from '@tanstack/react-query';

export const useGETPackageVersions = (registryName: string | undefined, packageName: string | undefined) => {
  const apiClient = useContextSelector(ApiClientContext, (v) => v?.apiClient);
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['GhcrPackageVersions', registryName, packageName],
    queryFn: ({ signal }) => apiClient?.api.imagesGetGhcrPackageVersions(registryName!, packageName!, { signal }),
    enabled: !!registryName && !!packageName,
  });

  return { data, error, isLoading, isSuccess };
};
