import { useParams } from 'react-router';
import { useGetContainerStats } from './hooks/useGetContainerStats';
import { ContainerStatView, ContainerView } from '@/api/_generated';
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
  const [stats, setStats] = useState<ContainerStatView[]>([]);

  useEffect(() => {
    if (isSuccess && data?.data) {
      setStats(data?.data.stats);
    }
  }, [isSuccess, data]);

  useEffect(() => {
    if (containersInfo && containerId) {
      const found = containersInfo.containers?.find((s) => s.containerId.startsWith(containerId));
      if (found) {
        setContainer(found);
        setStats((prev) => [...prev, found.lastStats]);
      }
    }
  }, [containersInfo, containerId]);

  useEffect(() => {}, []);

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
