import { useApiClientContext } from '@/api/ApiClientContext';
import { useQuery } from '@tanstack/react-query';

export const useGETDockerHubTags = (registryName: string | undefined, repositoryName: string | undefined) => {
  const { apiClient } = useApiClientContext();
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['GetDockerHubTags', registryName, repositoryName],
    queryFn: ({ signal }) =>
      apiClient?.api.imagesGetDockerHubRepositoryTags(registryName!, repositoryName!, { signal }),
    enabled: !!registryName && !!repositoryName,
  });

  return { data, error, isLoading, isSuccess };
};
