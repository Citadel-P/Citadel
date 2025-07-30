import { createContext } from 'react';
import { useRequiredContext } from '@/hooks/useRequiredContext';
import { ContainerStatView, ContainerView } from '@/api/_generated';

export interface ContainerStatsContextValue {
  isLoading: boolean;
  stats: ContainerStatView[];
  container: ContainerView | undefined;
}

export const ContainerStatsContext = createContext<ContainerStatsContextValue | undefined>(undefined);
ContainerStatsContext.displayName = 'ContainerStatsContext';

export const useContainerStatsContext = () => useRequiredContext(ContainerStatsContext);
