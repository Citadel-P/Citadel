import { ContainerView, DeleteContainersRequest } from '@/api/_generated';
import { useEffect, useState, useMemo, useCallback, createContext } from 'react';
import useContainersHub from './hooks/useContainersHub';
import { useParams } from 'react-router';
import { useDELETEContainers } from './hooks/useDELETEContainers';
import { toast } from 'sonner';
import { IDeleteDialogData, useDialogState } from '@/hooks/useDialogState';
import { useRequiredContext } from '@/hooks/useRequiredContext';

interface IContext {
  isLoading: boolean;
  platformId: string | undefined;
  containers: ContainerView[];
  dialogData: IDeleteDialogData<ContainerView>;
  setDialogData: (data: IDeleteDialogData<ContainerView>) => void;
  selectedRows: ContainerView[];
  setSelectedRows: (containers: ContainerView[]) => void;
  requestDelete: (data: DeleteContainersRequest) => void;
  deleteIsPending: boolean;
  onSearch: (searchTerm: string) => void;
}

export const ContainersContext = createContext<IContext | undefined>(undefined);

const ContainersProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const { platformId } = useParams<{ platformId: string }>();

  // Fetch containers data
  const { containersInfo, isLoading } = useContainersHub(platformId!);

  // Handle container deletion
  const { mutate, isSuccess: deleteIsSuccess, isPending: deleteIsPending } = useDELETEContainers();

  // State for selected rows and containers
  const [selectedRows, setSelectedRows] = useState<ContainerView[]>([]);
  const [containers, setContainers] = useState<ContainerView[]>([]);
  const [originalContainers, setOriginalContainers] = useState<ContainerView[]>([]);
  const [currentSearchTerm, setCurrentSearchTerm] = useState('');

  // Dialog state
  const { dialogData, setDialogData } = useDialogState<ContainerView>();

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
      const filtered = originalContainers.filter((container) => {
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

  // Handle successful deletion
  useEffect(() => {
    if (deleteIsSuccess) {
      setDialogData({ open: false });
      toast.success('The selected container(s) has been successfully deleted');
    }
  }, [deleteIsSuccess, setDialogData]);

  // Request to delete containers
  const requestDelete = useCallback(
    (data: DeleteContainersRequest) => {
      mutate(data);
    },
    [mutate],
  );

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
      setDialogData,
      selectedRows,
      setSelectedRows,
      requestDelete,
      deleteIsPending,
      onSearch,
    }),
    [
      isLoading,
      platformId,
      containers,
      dialogData,
      selectedRows,
      deleteIsPending,
      requestDelete,
      setDialogData,
      onSearch,
    ],
  );

  return <ContainersContext.Provider value={contextValue}>{children}</ContainersContext.Provider>;
};

export default ContainersProvider;
export const useContainersContext = () => useRequiredContext(ContainersContext);
