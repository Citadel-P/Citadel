import { useParams } from 'react-router';
import { useGetContainerStats } from './hooks/useGetContainerStats';
import { ContainerStatView, ContainerView } from '@/api/_generated';
import { createContext, useEffect, useMemo, useState } from 'react';
import { useRequiredContext } from '@/hooks/useRequiredContext';
import useContainersHub from '../../hooks/useContainersHub';
import { useAppContext } from '@/AppProvider';

interface IContext {
  isLoading: boolean;
  stats: ContainerStatView[];
  container: ContainerView | undefined;
}

export const ContainerStatsContext = createContext<IContext | undefined>(undefined);

const ContainerStatsProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const { currentContainer } = useAppContext();
  const { containersInfo } = useContainersHub(currentContainer?.platformId);
  const { containerId } = useParams<{ containerId: string }>();
  const { data, isSuccess, isLoading } = useGetContainerStats(containerId);
  const [container, setContainer] = useState<ContainerView>();
  // Extract stats from API response
  const stats = useMemo(() => (isSuccess ? data?.data.stats || [] : []), [isSuccess, data]);

  useEffect(() => {
    if (containersInfo && containerId) {
      const found = containersInfo.containers.find((s) => s.containerId.includes(containerId));
      if (found) {
        setContainer(found);
      }
    }
  }, [containersInfo, containerId]);

  const contextValue = useMemo(
    () => ({
      isLoading,
      container,
      stats,
    }),
    [isLoading, stats, container],
  );

  return <ContainerStatsContext.Provider value={contextValue}>{children}</ContainerStatsContext.Provider>;
};

export default ContainerStatsProvider;
export const useContainerStatsContext = () => useRequiredContext(ContainerStatsContext);
