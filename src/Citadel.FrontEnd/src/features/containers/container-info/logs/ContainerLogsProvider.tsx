import { useEffect, useState } from 'react';
import { useParams } from 'react-router';
import { usePOSTContainerLogs } from './hooks/usePOSTContainerLogs';
import { ContainerLogsContext } from './ContainerLogsContext';

export const ContainerLogsProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
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
