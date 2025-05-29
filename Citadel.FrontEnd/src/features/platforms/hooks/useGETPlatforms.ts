import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useQuery } from '@tanstack/react-query';

export const useGETPlatforms = () => {
  const apiClient = useContextSelector(ApiClientContext, (v) => v?.apiClient);
  const { data, error, isLoading, isSuccess, refetch } = useQuery({
    queryKey: ['useGETPlatforms'],
    queryFn: ({ signal }) => apiClient!.api.platformsList({ signal }),
  });

  return { data, error, isLoading, isSuccess, refetch };
};
