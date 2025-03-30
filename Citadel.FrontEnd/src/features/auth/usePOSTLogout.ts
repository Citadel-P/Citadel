import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useMutation } from '@tanstack/react-query';
import { useGetValidationErrors } from '@/hooks/useGetValidationErrors';

export const usePOSTLogout = () => {
  const apiClient = useContextSelector(ApiClientContext, (v) => v?.apiClient);
  const { mutate, isPending, isSuccess, error, data } = useMutation({
    mutationFn: apiClient?.api.authenticationLogout,
  });
  const validationErrors = useGetValidationErrors(error);

  return { mutate, isPending, data, isSuccess, validationErrors };
};
