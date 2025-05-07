import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useMutation } from '@tanstack/react-query';
import { useGetValidationErrors } from '@/hooks/useGetValidationErrors';

export const usePOSTVolume = () => {
  const apiClient = useContextSelector(ApiClientContext, (s) => s?.apiClient);
  const { mutate, isPending, isSuccess, error, data } = useMutation({ mutationFn: apiClient?.api.volumesCreate });
  const validationErrors = useGetValidationErrors(error);

  return { mutate, isPending, isSuccess, data, validationErrors };
};
