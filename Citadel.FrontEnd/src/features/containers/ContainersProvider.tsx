import { ContainerInfoView } from '@/api/_generated';
import { useRequiredContext } from '@/hooks/useRequiredContext';
import { createContext, useState } from 'react';
import useContainersHub from './hooks/useContainersHub';
import { useGETContainers } from './hooks/useGETContainers';
import { useParams } from 'react-router-dom';

interface IContext {
  isLoading: boolean;
  platformId: string | undefined;
  containers: ContainerInfoView[];
  selectedContainerIds: string[];
  onSelectionChange: (selectedRows: string[]) => void;
}
interface IProps {
  children?: React.ReactNode;
}

const ContainersContext = createContext<IContext | undefined>(undefined);

const ContainersProvider: React.FC<IProps> = ({ children }) => {
  const { platformId } = useParams();
  const { data, isLoading, isSuccess } = useGETContainers(platformId!);
  const { containersInfo } = useContainersHub(platformId!);
  const [selectedRowIds, setSelectedRowIds] = useState<string[]>([]);
  let containers: ContainerInfoView[] = [];

  if (isSuccess && data?.data) {
    containers = data.data.containers!;
  }

  if (containersInfo) {
    containers = containersInfo.containers ?? [];
  }

  const onSelectionChange = (rows: string[]) => setSelectedRowIds(rows);

  return (
    <ContainersContext.Provider
      value={{
        isLoading,
        platformId,
        containers,
        selectedContainerIds: selectedRowIds,
        onSelectionChange,
      }}>
      {children}
    </ContainersContext.Provider>
  );
};

export default ContainersProvider;

export const useContainersContext = () => useRequiredContext(ContainersContext);
