import { useParams } from 'react-router';
import { useGetContainerStats } from './hooks/useGetContainerStats';
import { ContainerStateStatus, ContainerStatView } from '@/api/generated/api.types';
import { useEffect, useMemo, useState } from 'react';
import { ContainerStatsContext } from './ContainerStatsContext';
import { DockerContainerView } from '@/api/models';

export const ContainerStatsProvider: React.FC<{
  children?: React.ReactNode;
  container: DockerContainerView | undefined;
}> = ({ children, container }) => {
  const { containerId } = useParams<{ containerId: string }>();
  const { data, isSuccess, isLoading } = useGetContainerStats(containerId);
  const [stats, setStats] = useState<ContainerStatView[]>([]);

  useEffect(() => {
    if (isSuccess && data?.data) {
      setStats(data?.data.stats);
    }
  }, [isSuccess, data]);

  useEffect(() => {
    if (containerId && container?.containerStat && container?.state === ContainerStateStatus.Running) {
      container.containerStat.created = Math.floor(Date.now() / 1000);
      setStats((prev) => [...prev, container.containerStat]);
    }
  }, [containerId, container]);

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
