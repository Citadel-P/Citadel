import { RegistryView } from '@/api/_generated';
import { useGETRegistries } from './hooks/useGETRegistries';
import { createContext, useEffect, useState } from 'react';
import { useDELETERegistries } from './hooks/useDELETERegistries';
import { toast } from 'sonner';
import { IDeleteDialogData, useDialogState } from '@/hooks/useDialogState';
import { useRequiredContext } from '@/hooks/useRequiredContext';

interface IContext {
  isLoading: boolean;
  registries: RegistryView[] | undefined;
  selectedRows: RegistryView[];
  setSelectedRows: (ids: RegistryView[]) => void;
  requestDelete: (ids: string[]) => void;
  deleteIsPending: boolean;
  dialogData: IDeleteDialogData<RegistryView>;
  setDialogData: (data: IDeleteDialogData<RegistryView>) => void;
}
interface IProps {
  children?: React.ReactNode;
}

export const RegistriesContext = createContext<IContext | undefined>(undefined);

const RegistriesProvider: React.FC<IProps> = ({ children }) => {
  const {
    mutate,
    isPending: deleteIsPending,
    data: deletedRegistries,
    isSuccess: deleteIsSuccess,
  } = useDELETERegistries();
  const { data, isLoading, isSuccess } = useGETRegistries();
  const [selectedRows, setSelectedRows] = useState<RegistryView[]>([]);
  const [registries, setRegistries] = useState<RegistryView[]>([]);
  const { dialogData, setDialogData } = useDialogState<RegistryView>();

  useEffect(() => {
    if (isSuccess && data?.data) {
      setRegistries(data?.data.registries ?? []);
    }
  }, [isSuccess, data]);

  useEffect(() => {
    if (deleteIsSuccess && dialogData.currentSelection) {
      const ids = dialogData.currentSelection?.map((s) => s.id);
      setRegistries((r) => r.filter((s) => !ids?.includes(s.id)));
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

export default RegistriesProvider;
export const useRegistriesContext = () => useRequiredContext(RegistriesContext);
