import { useState } from 'react';
import { useParams } from 'react-router';
import { useGetContainerStats } from './hooks/useGetContainerStats';
import { ContainerStatView } from '@/api/_generated';
import { createContext } from 'use-context-selector';

interface IContext {
  isLoading: boolean;
  requestId: string;
  stats: ContainerStatView[] | undefined;
}
interface IProps {
  children?: React.ReactNode;
}

export const ContainerStatsContext = createContext<IContext | undefined>(undefined);

const ContainerLogsProvider: React.FC<IProps> = ({ children }) => {
  const [requestId] = useState(crypto.randomUUID());
  const { containerId } = useParams();
  const { data, isSuccess, isLoading } = useGetContainerStats(containerId);
  let stats: ContainerStatView[] = [];
  if (isSuccess) {
    stats = data?.data.stats || [];
  }

  return (
    <ContainerStatsContext.Provider
      value={{
        isLoading,
        stats,
        requestId,
      }}>
      {children}
    </ContainerStatsContext.Provider>
  );
};

export default ContainerLogsProvider;
