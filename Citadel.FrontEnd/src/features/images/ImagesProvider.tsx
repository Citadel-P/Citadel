import { createContext } from 'use-context-selector';
import { useGETRegistries } from '../registries/hooks/useGETRegistries';
import { useEffect, useState, useCallback, useMemo } from 'react';
import { DeleteImagesRequest, ImageView, RegistryView } from '@/api/_generated';
import { useQueryClient } from '@tanstack/react-query';
import { useDELETEImages } from './hooks/useDELETEImages';
import { toast } from 'sonner';
import { IDeleteDialogData, useDialogState } from './hooks/useDialogState';

interface IContext {
  isLoading: boolean;
  registries: RegistryView[];
  setSelectionChange: (name: string) => void;
  selectedRegistry: RegistryView | undefined;
  selectedRows: ImageView[] | undefined;
  localImages: ImageView[];
  requestDelete: (request: DeleteImagesRequest) => void;
  setSelectedRows: (images: ImageView[] | undefined) => void;
  setLocalImages: (images: ImageView[]) => void;
  deleteIsPending: boolean;
  dialogData: IDeleteDialogData;
  setDialogData: (data: IDeleteDialogData) => void;
}

export const ImagesContext = createContext<IContext | undefined>(undefined);

const ImagesProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const client = useQueryClient();

  // Fetch registries
  const { data, isLoading, isSuccess } = useGETRegistries();

  // Delete images mutation
  const { mutate, isSuccess: deleteIsSuccess, isPending: deleteIsPending, data: deleteData } = useDELETEImages();

  // State variables
  const [registries, setRegistries] = useState<RegistryView[]>([]);
  const [selectedRows, setSelectedRows] = useState<ImageView[] | undefined>();
  const [selectedRegistry, setSelectedRegistry] = useState<RegistryView | undefined>();
  const [localImages, setLocalImages] = useState<ImageView[]>([]);
  const { dialogData, setDialogData } = useDialogState();

  // Update registries and selected registry when data is fetched
  useEffect(() => {
    if (isSuccess && data?.data) {
      const fetchedRegistries = data.data.registries ?? [];
      setRegistries(fetchedRegistries);
      setSelectedRegistry(fetchedRegistries[0] ?? undefined); // Select the first registry by default, or undefined if none
    }
  }, [isSuccess, data]);

  // Handle successful image deletion
  useEffect(() => {
    if (deleteIsSuccess) {
      client.invalidateQueries({ queryKey: ['getAllLocalImages'] });
      setDialogData({ open: false });

      const message =
        deleteData?.data?.replies && deleteData?.data?.replies.length > 1
          ? 'The selected images have been successfully deleted'
          : 'The selected image has been successfully deleted';

      toast.success(message);
    }
  }, [deleteIsSuccess, client, deleteData, setDialogData]);

  // Handle registry selection change
  const setSelectionChange = useCallback(
    (name: string) => {
      const registry = registries.find((s) => s.name === name);
      if (registry) {
        setSelectedRegistry(registry);
        client.invalidateQueries({ queryKey: ['externalRepositories', registry.name] });
      }
    },
    [registries, client],
  );

  // Handle image deletion request
  const requestDelete = useCallback(
    (request: DeleteImagesRequest) => {
      mutate(request);
    },
    [mutate],
  );

  // Memoized context value
  const contextValue = useMemo(
    () => ({
      isLoading,
      registries,
      selectedRegistry,
      selectedRows,
      setSelectedRows,
      setSelectionChange,
      requestDelete,
      localImages,
      setLocalImages,
      deleteIsPending,
      dialogData,
      setDialogData,
    }),
    [
      isLoading,
      registries,
      selectedRegistry,
      selectedRows,
      setSelectionChange,
      requestDelete,
      localImages,
      deleteIsPending,
      dialogData,
      setDialogData,
    ],
  );

  return <ImagesContext.Provider value={contextValue}>{children}</ImagesContext.Provider>;
};

export default ImagesProvider;
