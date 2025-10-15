import { ContainerView, DeleteContainersRequest } from '@/api/generated/api.types';
import { createContext } from 'react';
import { useRequiredContext } from '@/hooks/useRequiredContext';
import { DockerContainerView } from '@/api/types';
import { IDialogData } from '@/lib/hooks';

interface IContext {
  isLoading: boolean;
  platformId: string | undefined;
  containers: ContainerView[] | undefined;
  dialogData: IDialogData<ContainerView | DockerContainerView>;
  setDialogData: (data: IDialogData<ContainerView | DockerContainerView>) => void;
  selectedRows: ContainerView[] | undefined;
  setSelectedRows: (containers: ContainerView[] | undefined) => void;
  requestDelete: (data: DeleteContainersRequest) => void;
  deleteIsPending: boolean;
  onSearch: (searchTerm: string) => void;
}

export const ContainersContext = createContext<IContext | undefined>(undefined);
ContainersContext.displayName = 'ContainersContext';

export const useContainersContext = () => useRequiredContext(ContainersContext);
