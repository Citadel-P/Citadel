import { RegistryView } from '@/api/_generated';
import { createContext } from 'use-context-selector';
import { useGETRegistries } from './hooks/useGETRegistries';
import { useEffect, useState } from 'react';
import { useDELETERegistries } from './hooks/useDELETERegistries';
import { toast } from 'sonner';

interface IContext {
  isLoading: boolean;
  isPending: boolean;
  registries: RegistryView[] | undefined;
  selectedRowIds: string[];
  setSelectedRowIds: (ids: string[]) => void;
  requestDelete: (ids: string[]) => void;
}
interface IProps {
  children?: React.ReactNode;
}

export const RegistriesContext = createContext<IContext | undefined>(undefined);

const RegistriesProvider: React.FC<IProps> = ({ children }) => {
  const { mutate, isPending, data: deletedRegistries, isSuccess: deleteIsSuccess } = useDELETERegistries();
  const { data, isLoading, isSuccess } = useGETRegistries();
  const [selectedRowIds, setSelectedRowIds] = useState<string[]>([]);
  const [registries, setRegistries] = useState<RegistryView[]>([]);

  useEffect(() => {
    if (isSuccess && data?.data) {
      setRegistries(data?.data.registries ?? []);
    }
  }, [isSuccess, data]);

  useEffect(() => {
    if (deleteIsSuccess && deletedRegistries?.data) {
      const ids = deletedRegistries.data.registries?.map((s) => s.id) ?? [];
      setRegistries((r) => r.filter((s) => !ids.includes(s.id)));
      setSelectedRowIds([]);
      const message =
        deletedRegistries.data.registries!.length > 1
          ? 'The selected registries has been successfully deleted'
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
        isPending,
        registries,
        selectedRowIds,
        requestDelete,
        setSelectedRowIds,
      }}>
      {children}
    </RegistriesContext.Provider>
  );
};

export default RegistriesProvider;
