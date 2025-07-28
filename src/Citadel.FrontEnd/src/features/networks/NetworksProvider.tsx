import { useState, useMemo, useCallback, useEffect, createContext } from 'react';
import { DeleteNetworksInput, DockerNetworkResult } from '@/api/_generated';
import { IDeleteDialogData, useDialogState } from '@/hooks/useDialogState';
import { useDELETENetworks } from './hooks/useDELETENetworks';
import { useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { useRequiredContext } from '@/hooks/useRequiredContext';
import { use400ErrorToast } from '@/hooks/use400ErrorToast';

interface IContext {
  selectedRows: DockerNetworkResult[] | undefined;
  networks: DockerNetworkResult[];
  setSelectedRows: (networks: DockerNetworkResult[] | undefined) => void;
  setNetworks: (networks: DockerNetworkResult[]) => void;
  dialogData: IDeleteDialogData<DockerNetworkResult>;
  setDialogData: (data: IDeleteDialogData<DockerNetworkResult>) => void;
  onSearch: (searchTerm: string) => void;
  requestDelete: (request: DeleteNetworksInput) => void;
  deleteIsPending: boolean;
  sheetOpen: boolean;
  setSheetOpen: (open: boolean) => void;
  currentNetwork: DockerNetworkResult | undefined;
  setCurrentNetwork: (network: DockerNetworkResult | undefined) => void;
}

export const NetworksContext = createContext<IContext | undefined>(undefined);

const NetworksProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const client = useQueryClient();
  // State variables
  const [selectedRows, setSelectedRows] = useState<DockerNetworkResult[] | undefined>();
  const [networks, setNetworks] = useState<DockerNetworkResult[]>([]);
  const [originalNetworks, setOriginalNetworks] = useState<DockerNetworkResult[]>([]);
  // Sheet state
  const [currentNetwork, setCurrentNetwork] = useState<DockerNetworkResult>();
  const [sheetOpen, setSheetOpen] = useState(false);

  const { dialogData, setDialogData } = useDialogState<DockerNetworkResult>();
  const [currentSearchTerm, setCurrentSearchTerm] = useState('');

  const {
    mutate: deleteNetworks,
    isSuccess: deleteIsSuccess,
    isPending: deleteIsPending,
    error: deleteInError,
  } = useDELETENetworks();
  use400ErrorToast(
    deleteInError,
    'The selected network(s) could not be deleted (status code: 400).',
    on400ErrorHandled,
  );

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

  function on400ErrorHandled() {
    setDialogData({ open: false });
  }

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
export const useNetworksContext = () => useRequiredContext(NetworksContext);
