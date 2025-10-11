import { createContext } from 'react';
import { useRequiredContext } from '@/hooks/useRequiredContext';
import { ContainerStatView } from '@/api/generated/api.types';
import { DockerContainerView } from '@/api/models';

export interface ContainerStatsContextValue {
  isLoading: boolean;
  stats: ContainerStatView[];
  container: DockerContainerView | undefined;
}

export const ContainerStatsContext = createContext<ContainerStatsContextValue | undefined>(undefined);
ContainerStatsContext.displayName = 'ContainerStatsContext';

export const useContainerStatsContext = () => useRequiredContext(ContainerStatsContext);
