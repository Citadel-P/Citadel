import { createContext } from 'use-context-selector';
import { useState, useMemo, useCallback, useEffect } from 'react';
import { DeleteNetworksInput, DockerNetwork } from '@/api/_generated';
import { IDeleteDialogData, useDialogState } from '@/hooks/useDialogState';
import { useDELETENetworks } from './hooks/useDELETENetworks';
import { useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';

interface IContext {
  selectedRows: DockerNetwork[] | undefined;
  networks: DockerNetwork[];
  setSelectedRows: (networks: DockerNetwork[] | undefined) => void;
  setNetworks: (networks: DockerNetwork[]) => void;
  dialogData: IDeleteDialogData<DockerNetwork>;
  setDialogData: (data: IDeleteDialogData<DockerNetwork>) => void;
  onSearch: (searchTerm: string) => void;
  requestDelete: (request: DeleteNetworksInput) => void;
  deleteIsPending: boolean;
  sheetOpen: boolean;
  setSheetOpen: (open: boolean) => void;
  currentNetwork: DockerNetwork | undefined;
  setCurrentNetwork: (network: DockerNetwork | undefined) => void;
}

export const NetworksContext = createContext<IContext | undefined>(undefined);

const NetworksProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const client = useQueryClient();
  // State variables
  const [selectedRows, setSelectedRows] = useState<DockerNetwork[] | undefined>();
  const [networks, setNetworks] = useState<DockerNetwork[]>([]);
  const [originalNetworks, setOriginalNetworks] = useState<DockerNetwork[]>([]);
  // Sheet state
  const [currentNetwork, setCurrentNetwork] = useState<DockerNetwork>();
  const [sheetOpen, setSheetOpen] = useState(false);

  const { dialogData, setDialogData } = useDialogState<DockerNetwork>();
  const [currentSearchTerm, setCurrentSearchTerm] = useState('');

  const { mutate: deleteNetworks, isSuccess: deleteIsSuccess, isPending: deleteIsPending } = useDELETENetworks();

  // Wrapper function that handles both original and filtered networks
  const handleNetworksUpdate = useCallback((networks: DockerNetwork[]) => {
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
