import { createContext } from 'use-context-selector';
import { useState, useMemo, useCallback, useEffect } from 'react';
import { DeleteNetworksInput, NetworkView } from '@/api/_generated';
import { IDeleteDialogData, useDialogState } from '@/hooks/useDialogState';
import { useDELETENetworks } from './hooks/useDELETENetworks';
import { useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';

interface IContext {
  selectedRows: NetworkView[] | undefined;
  networks: NetworkView[];
  setSelectedRows: (networks: NetworkView[] | undefined) => void;
  setNetworks: (networks: NetworkView[]) => void;
  dialogData: IDeleteDialogData<NetworkView>;
  setDialogData: (data: IDeleteDialogData<NetworkView>) => void;
  onSearch: (searchTerm: string) => void;
  requestDelete: (request: DeleteNetworksInput) => void;
  deleteIsPending: boolean;
  sheetOpen: boolean;
  setSheetOpen: (open: boolean) => void;
  currentNetwork: NetworkView | undefined;
  setCurrentNetwork: (network: NetworkView | undefined) => void;
}

export const NetworksContext = createContext<IContext | undefined>(undefined);

const NetworksProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const client = useQueryClient();
  // State variables
  const [selectedRows, setSelectedRows] = useState<NetworkView[] | undefined>();
  const [networks, setNetworks] = useState<NetworkView[]>([]);
  const [originalNetworks, setOriginalNetworks] = useState<NetworkView[]>([]);
  // Sheet state
  const [currentNetwork, setCurrentNetwork] = useState<NetworkView>();
  const [sheetOpen, setSheetOpen] = useState(false);

  const { dialogData, setDialogData } = useDialogState<NetworkView>();
  const [currentSearchTerm, setCurrentSearchTerm] = useState('');

  const { mutate: deleteNetworks, isSuccess: deleteIsSuccess, isPending: deleteIsPending } = useDELETENetworks();

  // Wrapper function that handles both original and filtered networks
  const handleNetworksUpdate = useCallback((networks: NetworkView[]) => {
    setOriginalNetworks(networks);
    setNetworks(networks);
  }, []);

  // Search function to filter networks by name or ID
  const onSearch = useCallback((searchTerm: string) => {
    setCurrentSearchTerm(searchTerm);
  }, []);

  // Filter networks whenever search term changes
  useEffect(() => {
    if (!originalNetworks.length) return;

    if (currentSearchTerm.trim() === '') {
      // If no search term, show all images
      setNetworks(originalNetworks);
    } else {
      const searchLower = currentSearchTerm.toLowerCase();

      // Filter images by name OR id containing the search term
      const filtered = originalNetworks.filter((network) => {
        const nameMatches = network.name?.toLowerCase().includes(searchLower) || false;
        const idMatches =
          // Short ID format (first 12 characters)
          (network.id && network.id.substring(0, 12).toLowerCase().includes(searchLower)) ||
          // Full ID format
          (network.id && network.id.toLowerCase().includes(searchLower)) ||
          false;

        return nameMatches || idMatches;
      });

      setNetworks(filtered);
    }
  }, [originalNetworks, currentSearchTerm]);

  // Handle successful network deletion
  useEffect(() => {
    if (deleteIsSuccess) {
      client.invalidateQueries({ queryKey: ['useGETNetworks'] });
      setDialogData({ open: false });
      toast.success('The selected network(s) has been successfully deleted');
    }
  }, [deleteIsSuccess, client, setDialogData]);

  // Handle network deletion request
  const requestDelete = useCallback(
    (request: DeleteNetworksInput) => {
      deleteNetworks(request);
    },
    [deleteNetworks],
  );

  // Memoized context value
  const contextValue = useMemo(
    () => ({
      selectedRows,
      setSelectedRows,
      networks,
      setNetworks: handleNetworksUpdate,
      setDialogData,
      dialogData,
      onSearch,
      requestDelete,
      deleteIsPending,
      sheetOpen,
      setSheetOpen,
      currentNetwork,
      setCurrentNetwork,
    }),
    [
      selectedRows,
      deleteIsPending,
      setSelectedRows,
      networks,
      dialogData,
      setDialogData,
      onSearch,
      handleNetworksUpdate,
      requestDelete,
      sheetOpen,
      setSheetOpen,
      currentNetwork,
      setCurrentNetwork,
    ],
  );

  return <NetworksContext.Provider value={contextValue}>{children}</NetworksContext.Provider>;
};

export default NetworksProvider;
