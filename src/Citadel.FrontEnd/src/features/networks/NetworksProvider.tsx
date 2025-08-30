import { useState, useMemo, useCallback, useEffect } from 'react';
import { DockerNetworkResult } from '@/api/_generated';
import { NetworksContext } from './NetworksContext';
import { useDeleteNetworkDialog } from './hooks/useDeleteNetworkDialog';

export const NetworksProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  // State variables
  const [selectedRows, setSelectedRows] = useState<DockerNetworkResult[] | undefined>();
  const [networks, setNetworks] = useState<DockerNetworkResult[] | undefined>([]);
  const [originalNetworks, setOriginalNetworks] = useState<DockerNetworkResult[] | undefined>([]);
  const [currentNetwork, setCurrentNetwork] = useState<DockerNetworkResult>();
  const [currentSearchTerm, setCurrentSearchTerm] = useState('');

  const { setDialogData, dialogData, requestDelete, deleteIsPending } = useDeleteNetworkDialog();

  // Wrapper function that handles both original and filtered networks
  const handleNetworksUpdate = useCallback((networks: DockerNetworkResult[]) => {
    setOriginalNetworks(networks);
    setNetworks(networks);
  }, []);

  // Search function to filter networks by name or ID
  const onSearch = useCallback((searchTerm: string) => {
    setCurrentSearchTerm(searchTerm);
  }, []);

  // Filter networks whenever search term changes
  useEffect(() => {
    if (!originalNetworks?.length) return;

    if (currentSearchTerm.trim() === '') {
      // If no search term, show all networks
      setNetworks(originalNetworks);
    } else {
      const searchLower = currentSearchTerm.toLowerCase();

      // Filter networks by name OR id containing the search term
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
      currentNetwork,
      setCurrentNetwork,
    ],
  );

  return <NetworksContext.Provider value={contextValue}>{children}</NetworksContext.Provider>;
};
