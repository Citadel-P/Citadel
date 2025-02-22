import { ContainerInfoView } from '@/api/_generated';
import { useState } from 'react';
import { createContext } from 'use-context-selector';
import useContainersHub from './hooks/useContainersHub';
import { useGETContainers } from './hooks/useGETContainers';
import { useParams } from 'react-router';

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

export const ContainersContext = createContext<IContext | undefined>(undefined);

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
