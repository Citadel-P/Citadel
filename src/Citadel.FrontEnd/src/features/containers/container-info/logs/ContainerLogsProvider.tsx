import { useEffect, useState } from 'react';
import { ContainerLogsContext } from './ContainerLogsContext';
import { useContainerLogHub } from './hooks/useContainerLogHub';

export const ContainerLogsProvider: React.FC<{ children?: React.ReactNode; containerId: string | undefined }> = ({
  children,
  containerId,
}) => {
  const [logs, setLogs] = useState<string[] | undefined>(undefined);

  const { containerLog, containerLogs } = useContainerLogHub(containerId);
  const isPending = false;
  useEffect(() => {
    if (containerLog) {
      setLogs((prevChunks) => [...(prevChunks ?? []), containerLog]);
    }
  }, [containerLog]);

  useEffect(() => {
    if (containerLogs) {
      setLogs(containerLogs);
    }
  }, [containerLogs]);

  return (
    <ContainerLogsContext.Provider
      value={{
        isPending,
        logs: logs ?? [],
      }}>
      {children}
    </ContainerLogsContext.Provider>
  );
};
