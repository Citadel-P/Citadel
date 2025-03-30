import { useParams } from 'react-router';
import { useGetContainerStats } from './hooks/useGetContainerStats';
import { ContainerStatView } from '@/api/_generated';
import { createContext } from 'use-context-selector';
import { useMemo } from 'react';

interface IContext {
  isLoading: boolean;
  stats: ContainerStatView[];
}

// Create context for container stats
export const ContainerStatsContext = createContext<IContext | undefined>(undefined);

const ContainerStatsProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  // Get container ID from route params
  const { containerId } = useParams<{ containerId: string }>();

  // Fetch container stats
  const { data, isSuccess, isLoading } = useGetContainerStats(containerId);

  // Extract stats from API response
  const stats = useMemo(() => (isSuccess ? data?.data.stats || [] : []), [isSuccess, data]);

  // Memoize context value to prevent unnecessary re-renders
  const contextValue = useMemo(
    () => ({
      isLoading,
      stats,
    }),
    [isLoading, stats],
  );

  return <ContainerStatsContext.Provider value={contextValue}>{children}</ContainerStatsContext.Provider>;
};

export default ContainerStatsProvider;
