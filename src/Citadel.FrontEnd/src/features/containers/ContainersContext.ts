import { ContainerView, DeleteContainersRequest } from '@/api/_generated';
import { createContext } from 'react';
import { IDeleteDialogData } from '@/hooks/useDialogState';
import { useRequiredContext } from '@/hooks/useRequiredContext';

interface IContext {
  isLoading: boolean;
  platformId: string | undefined;
  containers: ContainerView[] | undefined;
  dialogData: IDeleteDialogData<ContainerView>;
  setDialogData: (data: IDeleteDialogData<ContainerView>) => void;
  selectedRows: ContainerView[] | undefined;
  setSelectedRows: (containers: ContainerView[] | undefined) => void;
  requestDelete: (data: DeleteContainersRequest) => void;
  deleteIsPending: boolean;
  onSearch: (searchTerm: string) => void;
}

export const ContainersContext = createContext<IContext | undefined>(undefined);
ContainersContext.displayName = 'ContainersContext';

export const useContainersContext = () => useRequiredContext(ContainersContext);
