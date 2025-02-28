import { ContainerInfoView } from '@/api/_generated';
import { useEffect, useState } from 'react';
import { createContext } from 'use-context-selector';
import useContainersHub from './hooks/useContainersHub';
import { useGETContainers } from './hooks/useGETContainers';
import { useParams } from 'react-router';

interface IContext {
  isLoading: boolean;
  platformId: string | undefined;
  containers: ContainerInfoView[];
  selectedContainerIds: string[];
  isPlatformOnline: boolean;
  setSelectedRowIds: (selectedRows: string[]) => void;
}
interface IProps {
  children?: React.ReactNode;
}

export const ContainersContext = createContext<IContext | undefined>(undefined);

const ContainersProvider: React.FC<IProps> = ({ children }) => {
  const { platformId } = useParams();
  const { data, isLoading, isSuccess } = useGETContainers(platformId!);
  const { containersInfo } = useContainersHub(platformId!);
  const [selectedRowIds, setSelectedRowIds] = useState<string[]>([]);
  const [containers, setContainers] = useState<ContainerInfoView[]>([]);
  const isPlatformOnline = containers.find((s) => s.state === 'offline') === undefined;

  useEffect(() => {
    if (isSuccess && data?.data) {
      setContainers(data.data.containers!);
    }
    if (containersInfo) {
      setContainers(containersInfo.containers ?? []);
    }
  }, [data, isSuccess, containersInfo]);

  return (
    <ContainersContext.Provider
      value={{
        isLoading,
        platformId,
        containers,
        selectedContainerIds: selectedRowIds,
        setSelectedRowIds,
        isPlatformOnline,
      }}>
      {children}
    </ContainersContext.Provider>
  );
};

export default ContainersProvider;
