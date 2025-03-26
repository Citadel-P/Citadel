import { RegistryView } from '@/api/_generated';
import { createContext } from 'use-context-selector';
import { useGETRegistries } from './hooks/useGETRegistries';
import { useEffect, useState } from 'react';
import { useDELETERegistries } from './hooks/useDELETERegistries';
import { toast } from 'sonner';

interface IContext {
  isLoading: boolean;
  registries: RegistryView[] | undefined;
  selectedRows: RegistryView[];
  setSelectedRows: (ids: RegistryView[]) => void;
  requestDelete: (ids: string[]) => void;
  deleteIsPending: boolean;
  dialogData: IDeleteDialogData;
  setDialogData: (data: IDeleteDialogData) => void;
}
interface IProps {
  children?: React.ReactNode;
}

interface IDeleteDialogData {
  open: boolean;
  currentSelection?: RegistryView[];
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
  const [dialogData, setDialogData] = useState<IDeleteDialogData>({ open: false });

  useEffect(() => {
    if (isSuccess && data?.data) {
      setRegistries(data?.data.registries ?? []);
    }
  }, [isSuccess, data]);

  useEffect(() => {
    if (deleteIsSuccess && deletedRegistries?.data) {
      const ids = deletedRegistries.data.registries?.map((s) => s.id) ?? [];
      setRegistries((r) => r.filter((s) => !ids.includes(s.id)));
      setDialogData({ open: false });
      setSelectedRows([]);
      const message =
        deletedRegistries.data.registries!.length > 1
          ? 'The selected registries have been successfully deleted'
          : 'The selected registry has been successfully deleted';
      toast.success(message);
    }
  }, [deleteIsSuccess, deletedRegistries]);

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
