import { useState, useMemo, useCallback, useEffect } from 'react';
import { DockerNetworkResult } from '@/api/generated/api.types';
import { NetworksContext } from './context';
import { useDeleteDialog } from '@/lib/hooks';

export const NetworksProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const [selectedRows, setSelectedRows] = useState<DockerNetworkResult[] | undefined>();
  const [networks, setNetworks] = useState<DockerNetworkResult[] | undefined>([]);
  const [originalNetworks, setOriginalNetworks] = useState<DockerNetworkResult[] | undefined>([]);
  const [currentNetwork, setCurrentNetwork] = useState<DockerNetworkResult>();
  const [currentSearchTerm, setCurrentSearchTerm] = useState('');

  const { setDialogData, dialogData, requestDelete, deleteIsPending } = useDeleteDialog<DockerNetworkResult>({
    type: 'Network',
  });

  const handleNetworksUpdate = useCallback((networks: DockerNetworkResult[]) => {
    setOriginalNetworks(networks);
    setNetworks(networks);
  }, []);

  const onSearch = useCallback((searchTerm: string) => {
    setCurrentSearchTerm(searchTerm);
  }, []);

  useEffect(() => {
    if (!originalNetworks?.length) return;

    if (currentSearchTerm.trim() === '') {
      setNetworks(originalNetworks);
    } else {
      const searchLower = currentSearchTerm.toLowerCase();

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
