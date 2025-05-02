import { createContext } from 'use-context-selector';
import { useState, useMemo, useCallback, useEffect } from 'react';
import { NetworkView } from '@/api/_generated';
import { IDeleteDialogData, useDialogState } from '@/hooks/useDialogState';

interface IContext {
  selectedRows: NetworkView[] | undefined;
  networks: NetworkView[];
  setSelectedRows: (networks: NetworkView[] | undefined) => void;
  setNetworks: (networks: NetworkView[]) => void;
  dialogData: IDeleteDialogData<NetworkView>;
  setDialogData: (data: IDeleteDialogData<NetworkView>) => void;
  onSearch: (searchTerm: string) => void;
  requestDelete: (request: any) => void;
  deleteIsPending: boolean;
}

export const NetworksContext = createContext<IContext | undefined>(undefined);

const NetworksProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  // State variables
  const [selectedRows, setSelectedRows] = useState<NetworkView[] | undefined>();
  const [networks, setNetworks] = useState<NetworkView[]>([]);
  const [originalNetworks, setOriginalNetworks] = useState<NetworkView[]>([]);

  const { dialogData, setDialogData } = useDialogState<NetworkView>();
  const [currentSearchTerm, setCurrentSearchTerm] = useState('');
  const deleteIsPending = false;

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

  // Handle image deletion request
  const requestDelete = useCallback((request: any) => {}, []);

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
    ],
  );

  return <NetworksContext.Provider value={contextValue}>{children}</NetworksContext.Provider>;
};

export default NetworksProvider;
