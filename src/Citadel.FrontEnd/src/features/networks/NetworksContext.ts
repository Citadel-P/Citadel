import { createContext } from 'react';
import { useRequiredContext } from '@/hooks/useRequiredContext';
import { IDialogData } from '@/hooks/useDialogState';
import { DeleteNetworksInput, DockerNetworkResult } from '@/api/generated/api.types';

interface IContext {
  selectedRows: DockerNetworkResult[] | undefined;
  networks: DockerNetworkResult[] | undefined;
  setSelectedRows: (networks: DockerNetworkResult[] | undefined) => void;
  setNetworks: (networks: DockerNetworkResult[]) => void;
  dialogData: IDialogData<DockerNetworkResult>;
  setDialogData: (data: IDialogData<DockerNetworkResult>) => void;
  onSearch: (searchTerm: string) => void;
  requestDelete: (request: DeleteNetworksInput) => void;
  deleteIsPending: boolean;
  currentNetwork: DockerNetworkResult | undefined;
  setCurrentNetwork: (network: DockerNetworkResult | undefined) => void;
}

export const NetworksContext = createContext<IContext | undefined>(undefined);
NetworksContext.displayName = 'NetworksContext';

export const useNetworksContext = () => useRequiredContext(NetworksContext);
