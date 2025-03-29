import { ContainerInfoView, DeleteContainersRequest } from '@/api/_generated';
import { useEffect, useState } from 'react';
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

interface IProps {
  children?: React.ReactNode;
}

export const ContainersContext = createContext<IContext | undefined>(undefined);

const ContainersProvider: React.FC<IProps> = ({ children }) => {
  const { platformId } = useParams<{ platformId: string }>();
  const { data, isLoading, isSuccess } = useGETContainers(platformId!);
  const { mutate, isSuccess: deleteIsSuccess, isPending: deleteIsPending } = useDELETEContainers();
  const { containersInfo } = useContainersHub(platformId!);
  const [selectedRows, setSelectedRows] = useState<ContainerInfoView[]>([]);

  const [containers, setContainers] = useState<ContainerInfoView[]>([]);
  const { dialogData, setDialogData } = useDialogState();

  useEffect(() => {
    if (isSuccess && data?.data) {
      setContainers(data.data.containers!);
    }
  }, [data, isSuccess]);

  useEffect(() => {
    if (containersInfo) {
      setContainers(containersInfo.containers ?? []);
    }
  }, [containersInfo]);

  useEffect(() => {
    if (deleteIsSuccess) {
      setDialogData({ open: false });
      toast.success('The selected container(s) has been successfully deleted');
    }
  }, [deleteIsSuccess, setDialogData]);

  function requestDelete(data: DeleteContainersRequest) {
    mutate(data);
  }

  return (
    <ContainersContext.Provider
      value={{
        isLoading,
        platformId,
        containers,
        dialogData,
        setDialogData,
        selectedRows,
        setSelectedRows,
        requestDelete,
        deleteIsPending,
      }}>
      {children}
    </ContainersContext.Provider>
  );
};

export default ContainersProvider;
