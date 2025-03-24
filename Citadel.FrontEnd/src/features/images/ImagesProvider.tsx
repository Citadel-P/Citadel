import { createContext } from 'use-context-selector';
import { useGETRegistries } from '../registries/hooks/useGETRegistries';
import { useEffect, useState } from 'react';
import { DeleteImagesRequest, ImageView, RegistryView } from '@/api/_generated';
import { useQueryClient } from '@tanstack/react-query';
import { useDELETEImages } from './hooks/useDELETEImages';
import { toast } from 'sonner';

interface IContext {
  isLoading: boolean;
  isPlatformOnline: boolean;
  registries: RegistryView[];
  setSelectionChange: (name: string) => void;
  selectedRegistry: RegistryView | undefined;
  selectedRowIds: string[];
  locaImages: ImageView[];
  requestDelete: (request: DeleteImagesRequest) => void;
  setSelectedRowIds: (ids: string[]) => void;
  setLocalImages: (images: ImageView[]) => void;
  deleteIsPending: boolean;
}
interface IProps {
  children?: React.ReactNode;
}

export const ImagesContext = createContext<IContext | undefined>(undefined);

const ImagesProvider: React.FC<IProps> = ({ children }) => {
  const client = useQueryClient();
  const { data, isLoading, isSuccess } = useGETRegistries();
  const {
    mutate,
    isSuccess: deleteIsSuccess,
    isPending: deleteIsPending,
    error: deleteInErrpr,
    data: deleteData,
  } = useDELETEImages();
  const [registries, setRegistries] = useState<RegistryView[]>([]);
  const [selectedRowIds, setSelectedRowIds] = useState<string[]>([]);
  const [selectedRegistry, setSelectedRegistry] = useState<RegistryView | undefined>();
  const [locaImages, setLocalImages] = useState<ImageView[]>([]);

  const isPlatformOnline = true;

  useEffect(() => {
    if (isSuccess && data?.data) {
      setRegistries(data?.data.registries ?? []);
      setSelectedRegistry(data?.data.registries?.at(0));
    }
  }, [isSuccess, data]);

  useEffect(() => {
    if (deleteIsSuccess) {
      client.invalidateQueries({ queryKey: ['getAllLocalImages'] });
      const message =
        deleteData?.data?.replies && deleteData?.data?.replies.length > 1
          ? 'The selected images has been successfully deleted'
          : 'The selected image has been successfully deleted';
      toast.success(message);
    }
  }, [deleteIsSuccess, client, deleteData]);

  function setSelectionChange(name: string) {
    const registry = registries.find((s) => s.name === name);
    if (registry) {
      setSelectedRegistry(registry);
      client.invalidateQueries({ queryKey: ['externalImages', registry.name] });
    }
  }

  function requestDelete(request: DeleteImagesRequest) {
    mutate(request);
  }

  return (
    <ImagesContext.Provider
      value={{
        isLoading,
        registries,
        isPlatformOnline,
        selectedRegistry,
        selectedRowIds,
        setSelectionChange,
        requestDelete,
        setSelectedRowIds,
        locaImages,
        setLocalImages,
        deleteIsPending,
      }}>
      {children}
    </ImagesContext.Provider>
  );
};

export default ImagesProvider;
