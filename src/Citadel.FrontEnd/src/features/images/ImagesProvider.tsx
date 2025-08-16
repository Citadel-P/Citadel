import { useGETRegistries } from '../registries/hooks/useGETRegistries';
import { useEffect, useState, useCallback, useMemo } from 'react';
import { DeleteImagesRequest, ImageView, RegistryView } from '@/api/_generated';
import { useQueryClient } from '@tanstack/react-query';
import { useDELETEImages } from './hooks/useDELETEImages';
import { toast } from 'sonner';
import { useDialogState } from '@/hooks/useDialogState';
import { use400ErrorToast } from '@/hooks/use400ErrorToast';
import { ImagesContext } from './ImagesContext';

export const ImagesProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const client = useQueryClient();
  const { data, isLoading, isSuccess } = useGETRegistries();
  const {
    mutate,
    isSuccess: deleteIsSuccess,
    isPending: deleteIsPending,
    data: deleteData,
    error: deleteInError,
  } = useDELETEImages();
  use400ErrorToast(deleteInError, 'The selected image(s) could not be deleted (status code: 400).', on400ErrorHandled);

  // State variables
  const [registries, setRegistries] = useState<RegistryView[]>([]);
  const [selectedRows, setSelectedRows] = useState<ImageView[] | undefined>();
  const [selectedRegistry, setSelectedRegistry] = useState<RegistryView | undefined>();
  const [localImages, setLocalImages] = useState<ImageView[]>([]);
  const [originalLocalImages, setOriginalLocalImages] = useState<ImageView[] | undefined>([]);
  const { dialogData, setDialogData } = useDialogState<ImageView>();
  const { dialogData: runDialogData, setDialogData: setRunDialogData } = useDialogState<ImageView>();
  
  const [currentSearchTerm, setCurrentSearchTerm] = useState('');
  // Sheet state
  const [currentImage, setCurrentImage] = useState<ImageView>();
  const [sheetOpen, setSheetOpen] = useState(false);

  // Update registries and selected registry when data is fetched
  useEffect(() => {
    if (isSuccess && data?.data) {
      const fetchedRegistries = data.data.registries ?? [];
      setRegistries(fetchedRegistries);
      setSelectedRegistry(fetchedRegistries[0] ?? undefined); // Select the first registry by default, or undefined if none
    }
  }, [isSuccess, data]);

  // Wrapper function that handles both original and filtered images
  const handleLocalImagesUpdate = useCallback((images: ImageView[]) => {
    setOriginalLocalImages(images);
    setLocalImages(images);
  }, []);

  // Filter images whenever search term changes
  useEffect(() => {
    if (!originalLocalImages?.length) return;

    if (currentSearchTerm.trim() === '') {
      // If no search term, show all images
      setLocalImages(originalLocalImages);
    } else {
      const searchLower = currentSearchTerm.toLowerCase();

      // Filter images by name OR id containing the search term
      const filtered = originalLocalImages.filter((image) => {
        const nameMatches =
          image.name?.toLowerCase().includes(searchLower) || image.tag?.toLowerCase().includes(searchLower) || false;
        const idMatches =
          // Short ID format (first 12 characters)
          (image.id && image.id.substring(0, 12).toLowerCase().includes(searchLower)) ||
          // Full ID format
          (image.id && image.id.toLowerCase().includes(searchLower)) ||
          false;

        return nameMatches || idMatches;
      });

      setLocalImages(filtered);
    }
  }, [originalLocalImages, currentSearchTerm]);

  // Handle successful image deletion
  useEffect(() => {
    if (deleteIsSuccess) {
      client.invalidateQueries({ queryKey: ['getAllLocalImages'] });
      setDialogData({ open: false });

      const message =
        deleteData?.data?.items && deleteData?.data?.items.length > 1
          ? 'The selected images have been successfully deleted'
          : 'The selected image has been successfully deleted';

      toast.success(message);
    }
  }, [deleteIsSuccess, client, deleteData, setDialogData]);

  function on400ErrorHandled() {
    setDialogData({ open: false });
  }

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

  // Search function to filter images by name or ID
  const onSearch = useCallback((searchTerm: string) => {
    setCurrentSearchTerm(searchTerm);
  }, []);

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
      setLocalImages: handleLocalImagesUpdate,
      deleteIsPending,
      dialogData,
      setDialogData,
      onSearch,
      currentImage,
      setCurrentImage,
      sheetOpen,
      setSheetOpen,
      runDialogData,
      setRunDialogData,
    }),
    [
      isLoading,
      registries,
      selectedRegistry,
      selectedRows,
      setSelectionChange,
      requestDelete,
      localImages,
      handleLocalImagesUpdate,
      deleteIsPending,
      dialogData,
      setDialogData,
      onSearch,
      currentImage,
      setCurrentImage,
      sheetOpen,
      setSheetOpen,
      runDialogData,
      setRunDialogData,
    ],
  );

  return <ImagesContext.Provider value={contextValue}>{children}</ImagesContext.Provider>;
};
