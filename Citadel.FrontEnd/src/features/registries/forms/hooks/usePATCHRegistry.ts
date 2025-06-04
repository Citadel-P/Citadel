import { ApiClientContext } from '@/api/ApiClientProvider';
import { useContextSelector } from 'use-context-selector';
import { useMutation } from '@tanstack/react-query';
import { useGetValidationErrors } from '@/hooks/useGetValidationErrors';
import { RegistryInput } from '@/api/_generated';
interface Props {
  id: string | undefined;
  data: RegistryInput;
}
export const usePATCHRegistry = () => {
  const apiClient = useContextSelector(ApiClientContext, (s) => s?.apiClient)!;
  const { mutate, isPending, isSuccess, error, data } = useMutation({
    mutationFn: ({ id, data }: Props) => {
      return apiClient.api.registriesPatch(id!, data);
    },
  });
  const validationErrors = useGetValidationErrors(error);

  return { mutate, isPending, isSuccess, data, validationErrors };
};
