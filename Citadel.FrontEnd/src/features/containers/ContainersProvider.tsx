import { ContainerInfoView, DeleteContainersRequest } from '@/api/_generated';
import { useEffect, useState, useMemo, useCallback } from 'react';
import { createContext } from 'use-context-selector';
import useContainersHub from './hooks/useContainersHub';
import { useGETContainers } from './hooks/useGETContainers';
import { useParams } from 'react-router';
import { useDELETEContainers } from './hooks/useDELETEContainers';
import { toast } from 'sonner';
import { IDeleteDialogData, useDialogState } from './hooks/useDialogState';

interface IContext {
  isLoading: boolean;
  platformId: string | undefined;
  containers: ContainerInfoView[];
  dialogData: IDeleteDialogData;
  setDialogData: (data: IDeleteDialogData) => void;
  selectedRows: ContainerInfoView[];
  setSelectedRows: (containers: ContainerInfoView[]) => void;
  requestDelete: (data: DeleteContainersRequest) => void;
  deleteIsPending: boolean;
}

export const ContainersContext = createContext<IContext | undefined>(undefined);

const ContainersProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const { platformId } = useParams<{ platformId: string }>();

  // Fetch containers data
  const { data, isLoading, isSuccess } = useGETContainers(platformId!);
  const { containersInfo } = useContainersHub(platformId!);

  // Handle container deletion
  const { mutate, isSuccess: deleteIsSuccess, isPending: deleteIsPending } = useDELETEContainers();

  // State for selected rows and containers
  const [selectedRows, setSelectedRows] = useState<ContainerInfoView[]>([]);
  const [containers, setContainers] = useState<ContainerInfoView[]>([]);

  // Dialog state
  const { dialogData, setDialogData } = useDialogState();

  // Update containers when data or hub info changes
  useEffect(() => {
    // order matter
    if (containersInfo?.containers) {
      setContainers(containersInfo.containers);
    } else if (isSuccess && data?.data?.containers) {
      setContainers(data.data.containers);
    }
  }, [data, isSuccess, containersInfo]);

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
    }),
    [isLoading, platformId, containers, dialogData, selectedRows, deleteIsPending, requestDelete, setDialogData],
  );

  return <ContainersContext.Provider value={contextValue}>{children}</ContainersContext.Provider>;
};

export default ContainersProvider;
