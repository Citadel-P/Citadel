import { useApiClientContext } from '@/api/ApiClientContext';
import { use400ErrorToast } from '@/hooks/use400ErrorToast';
import { useQuery } from '@tanstack/react-query';

export const useGETInspect = (platformId: string | null, networkId: string | null) => {
  const { apiClient } = useApiClientContext();
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['useGETInspectNetwork', platformId, networkId],
    queryFn: ({ signal }) => apiClient?.api.networksInspect(platformId!, networkId!, { signal }),
    enabled: !!platformId && !!networkId,
  });
  use400ErrorToast(error);
  return { data, error, isLoading, isSuccess };
};
