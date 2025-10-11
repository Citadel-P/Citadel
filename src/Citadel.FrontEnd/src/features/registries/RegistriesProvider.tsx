import { DeleteRegistriesInput, RegistryView } from '@/api/generated/api.types';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { toast } from 'sonner';
import { useDialogState } from '@/hooks/useDialogState';
import { RegistriesContext } from './RegistriesContext';
import { useQueryClient } from '@tanstack/react-query';
import { useMutate, useRead } from '@/lib/hooks';

export const RegistriesProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const client = useQueryClient();
  const { mutate, isPending: deleteIsPending, isSuccess: deleteIsSuccess } = useMutate('deleteRegistries');
  const { data, isLoading, isSuccess } = useRead('listRegistries');

  const [selectedRows, setSelectedRows] = useState<RegistryView[] | undefined>([]);
  const [registries, setRegistries] = useState<RegistryView[] | undefined>([]);
  const { dialogData, setDialogData } = useDialogState<RegistryView>();
  useEffect(() => {
    if (isSuccess && data?.data) {
      setRegistries(data?.data.registries ?? []);
    }
  }, [isSuccess, data]);

  useEffect(() => {
    if (deleteIsSuccess) {
      client.invalidateQueries({ queryKey: ['listRegistries'] });
      setDialogData({ open: false });
      setSelectedRows([]);

      toast.success('The selected registry(s) has been successfully deleted');
    }
  }, [deleteIsSuccess, client, setDialogData]);

  const requestDelete = useCallback(
    (ids: string[]) => {
      mutate({ ids } as DeleteRegistriesInput);
    },
    [mutate],
  );

  const contextValue = useMemo(
    () => ({
      isLoading,
      deleteIsPending,
      registries,
      selectedRows,
      requestDelete,
      setSelectedRows,
      dialogData,
      setDialogData,
    }),
    [isLoading, deleteIsPending, registries, selectedRows, requestDelete, setSelectedRows, dialogData, setDialogData],
  );

  return <RegistriesContext.Provider value={contextValue}>{children}</RegistriesContext.Provider>;
};
