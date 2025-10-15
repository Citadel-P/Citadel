import { useEffect, useState, useCallback, useMemo } from 'react';
import { ImageView, RegistryView } from '@/api/generated/api.types';
import { useQueryClient } from '@tanstack/react-query';
import { ImagesContext } from './ImagesContext';
import { useRunImageDialog } from './hooks/useRunImageDialog';
import { useImagesGroup } from './hooks/useImagesGroup';
import { useParams } from 'react-router';
import { useDeleteDialog, useRead } from '@/lib/hooks';

export const ImagesProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const client = useQueryClient();
  const { platformId } = useParams<{ platformId: string }>();
  const { data, isLoading, isSuccess } = useRead('listRegistries');

  const { imagesInfo, isLoading: imagesLoading } = useImagesGroup(platformId);

  // State variables
  const [registries, setRegistries] = useState<RegistryView[]>([]);
  const [selectedRows, setSelectedRows] = useState<ImageView[] | undefined>();
  const [selectedRegistry, setSelectedRegistry] = useState<RegistryView | undefined>();
  const [localImages, setLocalImages] = useState<ImageView[]>([]);
  const [originalLocalImages, setOriginalLocalImages] = useState<ImageView[] | undefined>([]);
  const { deleteIsPending, requestDelete, dialogData, setDialogData } = useDeleteDialog<ImageView>({
    type: 'Image',
  });
  const { runDialogData, setRunDialogData } = useRunImageDialog();

  const [currentSearchTerm, setCurrentSearchTerm] = useState('');
  // Sheet state
  const [currentImage, setCurrentImage] = useState<ImageView>();

  // Update registries and selected registry when data is fetched
  useEffect(() => {
    if (isSuccess && data?.data) {
      const fetchedRegistries = data.data.registries ?? [];
      setRegistries(fetchedRegistries);
      setSelectedRegistry(fetchedRegistries[0] ?? undefined); // Select the first registry by default, or undefined if none
    }
  }, [isSuccess, data]);

  // Update containers when hub info changes
  useEffect(() => {
    if (imagesInfo?.images) {
      setOriginalLocalImages(imagesInfo.images);
    }
  }, [imagesInfo]);

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
          (image.imageId && image.imageId.substring(0, 12).toLowerCase().includes(searchLower)) ||
          // Full ID format
          (image.imageId && image.imageId.toLowerCase().includes(searchLower)) ||
          false;

        return nameMatches || idMatches;
      });

      setLocalImages(filtered);
    }
  }, [originalLocalImages, currentSearchTerm]);

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

  // Search function to filter images by name or ID
  const onSearch = useCallback((searchTerm: string) => {
    setCurrentSearchTerm(searchTerm);
  }, []);

  // Memoized context value
  const contextValue = useMemo(
    () => ({
      isLoading: imagesLoading || isLoading,
      registries,
      selectedRegistry,
      selectedRows,
      setSelectedRows,
      setSelectionChange,
      requestDelete,
      localImages,
      deleteIsPending,
      dialogData,
      setDialogData,
      onSearch,
      currentImage,
      setCurrentImage,
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
      deleteIsPending,
      dialogData,
      setDialogData,
      onSearch,
      currentImage,
      setCurrentImage,
      runDialogData,
      setRunDialogData,
      imagesLoading,
    ],
  );

  return <ImagesContext.Provider value={contextValue}>{children}</ImagesContext.Provider>;
};
