import { ContainerView } from '@/api/generated/api.types';
import { useEffect, useState, useMemo, useCallback } from 'react';
import { useContainersGroup } from './hooks/useContainersGroup';
import { useParams } from 'react-router';
import { ContainersContext } from './ContainersContext';
import { DockerContainerView } from '@/api/types';
import { useDeleteDialog } from '@/lib/hooks';

export const ContainersProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const { platformId } = useParams<{ platformId: string }>();

  // Fetch containers data
  const { containersInfo, isLoading } = useContainersGroup(platformId);

  // State for selected rows and containers
  const [selectedRows, setSelectedRows] = useState<ContainerView[] | undefined>([]);
  const [containers, setContainers] = useState<ContainerView[] | undefined>([]);
  const [originalContainers, setOriginalContainers] = useState<ContainerView[] | undefined>([]);
  const [currentSearchTerm, setCurrentSearchTerm] = useState('');

  const { deleteIsPending, requestDelete, dialogData, setDialogData } = useDeleteDialog<ContainerView | DockerContainerView>({
      type: 'Container',
    });
  // Update containers when hub info changes
  useEffect(() => {
    if (containersInfo?.containers) {
      setOriginalContainers(containersInfo.containers);
    }
  }, [containersInfo]);

  // Filter containers whenever original containers or search term changes
  useEffect(() => {
    if (currentSearchTerm.trim() === '') {
      setContainers(originalContainers);
    } else {
      const searchLower = currentSearchTerm.toLowerCase();

      // Filter containers by name OR containerId containing the search term
      const filtered = originalContainers?.filter((container) => {
        // Check container name (if it exists)
        const nameMatches = container.name?.toLowerCase().includes(searchLower) || false;

        // Use either short containerId format or full containerId format
        const idMatches =
          // Short containerId format (first 12 characters)
          (container.containerId && container.containerId.substring(0, 12).toLowerCase().includes(searchLower)) ||
          // Full containerId format
          (container.containerId && container.containerId.toLowerCase().includes(searchLower)) ||
          false;

        // Return true if either name or containerId matches
        return nameMatches || idMatches;
      });

      setContainers(filtered);
    }
  }, [originalContainers, currentSearchTerm]);

  // Search function to filter containers by name
  const onSearch = useCallback((searchTerm: string) => {
    setCurrentSearchTerm(searchTerm);
  }, []);

  // Memoize context value to prevent unnecessary re-renders
  const contextValue = useMemo(
    () => ({
      isLoading,
      platformId,
      containers,
      dialogData,
      deleteIsPending,
      setDialogData,
      selectedRows,
      setSelectedRows,
      requestDelete,
      onSearch,
    }),
    [
      isLoading,
      platformId,
      containers,
      selectedRows,
      dialogData,
      deleteIsPending,
      requestDelete,
      setDialogData,
      onSearch,
    ],
  );

  return <ContainersContext.Provider value={contextValue}>{children}</ContainersContext.Provider>;
};
