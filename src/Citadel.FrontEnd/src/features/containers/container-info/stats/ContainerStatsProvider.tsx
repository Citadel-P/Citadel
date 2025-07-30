import { useParams } from 'react-router';
import { useGetContainerStats } from './hooks/useGetContainerStats';
import { ContainerView } from '@/api/_generated';
import { useEffect, useMemo, useState } from 'react';
import useContainersHub from '../../hooks/useContainersHub';
import { useAppContext } from '@/AppContext';
import { ContainerStatsContext } from './ContainerStatsContext';

export const ContainerStatsProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const { currentContainer } = useAppContext();
  const { containersInfo } = useContainersHub(currentContainer?.platformId);
  const { containerId } = useParams<{ containerId: string }>();
  const { data, isSuccess, isLoading } = useGetContainerStats(containerId);
  const [container, setContainer] = useState<ContainerView | undefined>();

  const stats = useMemo(() => (isSuccess ? data?.data.stats || [] : []), [isSuccess, data]);

  useEffect(() => {
    if (containersInfo && containerId) {
      const found = containersInfo.containers?.find((s) => s.containerId.startsWith(containerId));
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
