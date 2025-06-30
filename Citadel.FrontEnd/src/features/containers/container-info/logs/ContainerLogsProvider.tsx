import { createContext, useEffect, useState } from 'react';
import { useParams } from 'react-router';
import { usePOSTContainerLogs } from './hooks/usePOSTContainerLogs';
import { useRequiredContext } from '@/hooks/useRequiredContext';

interface IContext {
  isPending: boolean;
  isSuccess: boolean;
  logs: string[];
}
interface IProps {
  children?: React.ReactNode;
}

export const ContainerLogsContext = createContext<IContext | undefined>(undefined);

const ContainerLogsProvider: React.FC<IProps> = ({ children }) => {
  const { containerId } = useParams();
  const [logs, setLogs] = useState<string[] | undefined>(undefined);
  const handleChunkReceived = (chunk: string) => {
    setLogs((prevChunks) => [...(prevChunks ?? []), chunk]);
  };
  const { mutate, isPending, isSuccess } = usePOSTContainerLogs(handleChunkReceived);

  useEffect(() => {
    const controller = new AbortController();
    if (containerId) {
      mutate({ containerId, signal: controller.signal });
    }
    return () => {
      controller.abort();
    };
  }, [containerId, mutate]);

  return (
    <ContainerLogsContext.Provider
      value={{
        isPending,
        isSuccess,
        logs: logs ?? [],
      }}>
      {children}
    </ContainerLogsContext.Provider>
  );
};

export default ContainerLogsProvider;
export const useContainerLogsContext = () => useRequiredContext(ContainerLogsContext);
