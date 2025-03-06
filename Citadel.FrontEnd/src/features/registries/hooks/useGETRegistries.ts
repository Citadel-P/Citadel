import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useQuery } from '@tanstack/react-query';

export const useGETRegistries = () => {
  const apiClient = useContextSelector(ApiClientContext, (v) => v?.apiClient);
  const { data, error, isLoading, isSuccess } = useQuery({
    queryKey: ['getAllRegistries'],
    queryFn: ({ signal }) => apiClient!.api.registriesGetAll({ signal }),
  });

  return { data, error, isLoading, isSuccess };
};
