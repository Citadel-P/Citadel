import { useEffect, useState } from 'react';
import useContainerLogsHub from './hooks/useContainerLogsHub';
import { useParams } from 'react-router';
import { usePOSTContainerLogs } from './hooks/usePOSTContainerLogs';
import { RequestedLogAction } from '@/api/_generated';
import { createContext } from 'use-context-selector';

interface IContext {
  isPending: boolean;
  isSuccess: boolean;
  logs: string[];
  requestId: string;
}
interface IProps {
  children?: React.ReactNode;
}

export const ContainerLogsContext = createContext<IContext | undefined>(undefined);

const ContainerLogsProvider: React.FC<IProps> = ({ children }) => {
  const [requestId] = useState(crypto.randomUUID());
  const { containerId } = useParams();
  const { mutate, isPending, isSuccess } = usePOSTContainerLogs();
  const { containerLogsMessage } = useContainerLogsHub(containerId!, requestId);
  const logs: string[] = [];

  if (containerLogsMessage) {
    logs.push(containerLogsMessage.log);
  }

  useEffect(() => {
    // https://react.dev/learn/synchronizing-with-effects#putting-it-all-together
    function onTimeout() {
      if (containerId)
        mutate({ requestParams: { containerId, requestId, requestedLogAction: RequestedLogAction.START } });
    }
    const timeoutId = setTimeout(onTimeout, 1000);

    return () => {
      if (containerId)
        mutate({ requestParams: { containerId, requestId, requestedLogAction: RequestedLogAction.STOP } });
      clearTimeout(timeoutId);
    };
  }, [containerId, requestId, mutate]);

  return (
    <ContainerLogsContext.Provider
      value={{
        isPending,
        isSuccess,
        logs,
        requestId,
      }}>
      {children}
    </ContainerLogsContext.Provider>
  );
};

export default ContainerLogsProvider;
