import { ContainerLogsContext } from './ContainerLogsContext';
import { useContainerLogGroup } from './hooks/useContainerLogGroup';

export const ContainerLogsProvider: React.FC<{ children?: React.ReactNode; containerId: string | undefined }> = ({
  children,
  containerId,
}) => {
  const { containerLogs } = useContainerLogGroup(containerId);

  return (
    <ContainerLogsContext.Provider
      value={{
        logs: containerLogs ?? [],
      }}>
      {children}
    </ContainerLogsContext.Provider>
  );
};
