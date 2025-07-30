import { RegistryView } from '@/api/_generated';
import { useGETRegistries } from './hooks/useGETRegistries';
import { useEffect, useState } from 'react';
import { useDELETERegistries } from './hooks/useDELETERegistries';
import { toast } from 'sonner';
import { useDialogState } from '@/hooks/useDialogState';
import { RegistriesContext } from './RegistriesContext';

export const RegistriesProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const {
    mutate,
    isPending: deleteIsPending,
    data: deletedRegistries,
    isSuccess: deleteIsSuccess,
  } = useDELETERegistries();
  const { data, isLoading, isSuccess } = useGETRegistries();
  const [selectedRows, setSelectedRows] = useState<RegistryView[] | undefined>([]);
  const [registries, setRegistries] = useState<RegistryView[] | undefined>([]);
  const { dialogData, setDialogData } = useDialogState<RegistryView>();

  useEffect(() => {
    if (isSuccess && data?.data) {
      setRegistries(data?.data.registries ?? []);
    }
  }, [isSuccess, data]);

  useEffect(() => {
    if (deleteIsSuccess && dialogData.currentSelection) {
      const ids = dialogData.currentSelection?.map((s) => s.id);
      setRegistries((r) => r?.filter((s) => !ids?.includes(s.id)));
      setDialogData({ open: false, currentSelection: undefined });
      setSelectedRows([]);

      toast.success('The selected registry(s) has been successfully deleted');
    }
  }, [deleteIsSuccess, deletedRegistries, dialogData.currentSelection, setDialogData]);

  const requestDelete = (ids: string[]) => {
    mutate({ ids });
  };

  return (
    <RegistriesContext.Provider
      value={{
        isLoading,
        deleteIsPending,
        registries,
        selectedRows,
        requestDelete,
        setSelectedRows,
        dialogData,
        setDialogData,
      }}>
      {children}
    </RegistriesContext.Provider>
  );
};
