import { RegistryView } from '@/api/generated/api.types';
import { useEffect, useMemo, useState } from 'react';
import { RegistriesContext } from './RegistriesContext';
import { useDeleteDialog, useRead } from '@/lib/hooks';

export const RegistriesProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const { data, isLoading, isSuccess } = useRead('listRegistries');

  const [selectedRows, setSelectedRows] = useState<RegistryView[] | undefined>([]);
  const [registries, setRegistries] = useState<RegistryView[] | undefined>([]);
  const { setDialogData, dialogData, requestDelete, deleteIsPending } = useDeleteDialog<RegistryView>({
    type: 'Registry',
  });

  useEffect(() => {
    if (isSuccess && data?.data) {
      setRegistries(data?.data.registries ?? []);
    }
  }, [isSuccess, data]);

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
