import { ContainerInfoView, DeleteContainersRequest } from '@/api/_generated';
import { useEffect, useState } from 'react';
import { createContext } from 'use-context-selector';
import useContainersHub from './hooks/useContainersHub';
import { useGETContainers } from './hooks/useGETContainers';
import { useParams } from 'react-router';
import { useDELETEContainers } from './hooks/useDELETEContainers';
import { toast } from 'sonner';

interface IContext {
  isLoading: boolean;
  platformId: string | undefined;
  containers: ContainerInfoView[];
  isPlatformOnline: boolean;
  dialogData: IDeleteDialogData;
  setDialogData: (data: IDeleteDialogData) => void;
  selectedRows: ContainerInfoView[];
  setSelectedRows: (containers: ContainerInfoView[]) => void;
  requestDelete: (data: DeleteContainersRequest) => void;
  deleteIsPending: boolean;
}

interface IDeleteDialogData {
  open: boolean;
  currentSelection?: ContainerInfoView[];
}

interface IProps {
  children?: React.ReactNode;
}

export const ContainersContext = createContext<IContext | undefined>(undefined);

const ContainersProvider: React.FC<IProps> = ({ children }) => {
  const { platformId } = useParams();
  const { data, isLoading, isSuccess } = useGETContainers(platformId!);
  const { mutate, isSuccess: deleteIsSuccess, isPending: deleteIsPending } = useDELETEContainers();
  const { containersInfo } = useContainersHub(platformId!);
  const [selectedRows, setSelectedRows] = useState<ContainerInfoView[]>([]);

  const [containers, setContainers] = useState<ContainerInfoView[]>([]);
  const [dialogData, setDialogData] = useState<IDeleteDialogData>({ open: false });
  const isPlatformOnline = containers.find((s) => s.state === 'offline') === undefined;
  useEffect(() => {
    if (isSuccess && data?.data) {
      setContainers(data.data.containers!);
    }
    if (containersInfo) {
      setContainers(containersInfo.containers ?? []);
    }
  }, [data, isSuccess, containersInfo]);

  useEffect(() => {
    if (deleteIsSuccess) {
      setDialogData({ open: false });
      toast.success('The selected container(s) has been successfully deleted');
    }
  }, [deleteIsSuccess]);

  function requestDelete(data: DeleteContainersRequest) {
    mutate(data);
  }

  return (
    <ContainersContext.Provider
      value={{
        isLoading,
        platformId,
        containers,
        isPlatformOnline,
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
