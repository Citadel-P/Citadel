import { useRequiredContext } from '@/hooks/useRequiredContext';
import { createContext, useEffect, useState } from 'react';
import useContainerLogsHub from './hooks/useContainerLogsHub';
import { useParams } from 'react-router-dom';
import { usePOSTContainerLogs } from './hooks/usePOSTContainerLogs';
import { RequestedLogAction } from '@/api/_generated';

interface IContext {
  isPending: boolean;
  logs: string[];
  requestId: string;
}
interface IProps {
  children?: React.ReactNode;
}

const ContainerLogsContext = createContext<IContext | undefined>(undefined);

const ContainerLogsProvider: React.FC<IProps> = ({ children }) => {
  const [requestId] = useState(crypto.randomUUID());
  const { containerId } = useParams();
  const { mutate, isPending } = usePOSTContainerLogs();
  const { containerLogsMessage } = useContainerLogsHub(containerId!, requestId);
  let logs: string[] = [];

  if (containerLogsMessage) {
    logs = containerLogsMessage.messages;
  }

  useEffect(() => {
    if (!containerId) return;
    // https://react.dev/learn/synchronizing-with-effects#putting-it-all-together
    function onTimeout() {
      mutate({ requestParams: { containerId, requestId, requestedLogAction: RequestedLogAction.START } });
    }
    const timeoutId = setTimeout(onTimeout, 1000);

    return () => {
      mutate({ requestParams: { containerId, requestId, requestedLogAction: RequestedLogAction.STOP } });
      clearTimeout(timeoutId);
    };
  }, [containerId, requestId, mutate]);

  return (
    <ContainerLogsContext.Provider
      value={{
        isPending,
        logs,
        requestId,
      }}>
      {children}
    </ContainerLogsContext.Provider>
  );
};

export default ContainerLogsProvider;

export const useContainerLogsContext = () => useRequiredContext(ContainerLogsContext);
