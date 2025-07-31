import { useRequiredContext } from '@/hooks/useRequiredContext';
import { createContext } from 'react';

interface IContext {
  isPending: boolean;
  isSuccess: boolean;
  logs: string[];
}

export const ContainerLogsContext = createContext<IContext | undefined>(undefined);
ContainerLogsContext.displayName = 'ContainerLogsContext';

export const useContainerLogsContext = () => useRequiredContext(ContainerLogsContext);
