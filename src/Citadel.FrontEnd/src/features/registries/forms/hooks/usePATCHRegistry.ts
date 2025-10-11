import { useApiClientContext } from '@/api/ApiClientContext';
import { useMutation } from '@tanstack/react-query';
import { useGetValidationErrors } from '@/hooks/useGetValidationErrors';
import { RegistryInput } from '@/api/generated/api.types';
interface Props {
  id: string | undefined;
  data: RegistryInput;
}
export const usePATCHRegistry = () => {
  const { apiClient } = useApiClientContext();
  const { mutate, isPending, isSuccess, error, data } = useMutation({
    mutationFn: ({ id, data }: Props) => {
      return apiClient.api.updateRegistry(id!, data);
    },
  });
  const validationErrors = useGetValidationErrors(error);

  return { mutate, isPending, isSuccess, data, validationErrors };
};
