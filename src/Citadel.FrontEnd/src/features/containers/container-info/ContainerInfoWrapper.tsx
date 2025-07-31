import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { ContainerLogsProvider } from './logs/ContainerLogsProvider';
import ContainerLogs from './logs/ContainerLogs';
import { useAppContext } from '@/AppContext';
import { useNavigate } from 'react-router';
import NetworkUsage from './stats/NetworkUsage';
import MemoryUsage from './stats/MemoryUsage';
import CpuUsage from './stats/CpuUsage';
import { useMemo, useCallback, useEffect, useState } from 'react';
import ContainerInspect from './inspect/ContainerInspect';
import Loader from '@/components/ui/loader';
import { ContainerStatsProvider } from './stats/ContainerStatsProvider';
import useContainerInfoHub from '../hooks/useContainerInfoHub';
import { ContainerStateStatus } from '@/api/_generated';
import { ContainerStateIndicator } from '../ContainerStateIndicator';

const ContainerInfoWrapper = () => {
  const navigate = useNavigate();
  const { route, currentContainer, isLoading } = useAppContext();
  const { containerInfo } = useContainerInfoHub(currentContainer?.containerId);

  const [containerId, setContainerId] = useState<string | undefined>();
  const [containerName, setContainerName] = useState<string | undefined>();
  const [containerState, setContainerState] = useState<ContainerStateStatus | undefined>();

  useEffect(() => {
    console.log(currentContainer, containerInfo);
    setContainerName(containerInfo?.name ?? currentContainer?.containerName);
    setContainerId(containerInfo?.containerId ?? currentContainer?.containerId);
    setContainerState(containerInfo?.state ?? currentContainer?.state);
  }, [currentContainer, containerInfo]);

  // Memoize the current tab based on the route
  const currentTab = useMemo(() => {
    const matches = route?.path.match('[^/]+$');
    const tab = matches && matches[0];
    return tab && ['logs', 'stats', 'inspect'].includes(tab) ? tab : 'logs';
  }, [route]);

  // Handle tab change
  const onValueChange = useCallback(
    (tabName: string) => {
      if (containerInfo?.containerId) {
        navigate(`../containers/${containerInfo.containerId.slice(0, 12)}/${tabName}`);
      }
    },
    [navigate, containerInfo],
  );

  if (isLoading) return <Loader />;

  // Render a fallback if currentContainer is undefined
  if (!containerId) {
    return (
      <div className="flex justify-center items-center h-full">
        <p className="text-muted-foreground">No container selected. Please select a container to view details.</p>
      </div>
    );
  }

  return (
    <div className="flex-col justify-between">
      <div className="px-4 py-4 lg:container sm:px-6 mx-auto">
        <div className="max-w-full rounded-lg border-border bg-background p-4">
          {/* Header */}
          <div className="flex items-baseline gap-1 mb-3">
            <ContainerStateIndicator stat={containerState ?? ContainerStateStatus.Exited} />
            <div className="text-md font-bold text-foreground">
              <span>{containerName?.slice(1)}</span>
              <span className="text-sm text-foreground/40 ml-2">({containerId?.slice(0, 12)})</span>
            </div>
          </div>

          {/* Tabs */}
          <Tabs value={currentTab} onValueChange={onValueChange}>
            <TabsList className="w-full justify-start bg-muted/20 rounded-sm">
              <TabsTrigger value="logs">Logs</TabsTrigger>
              <TabsTrigger value="inspect">Inspect</TabsTrigger>
              <TabsTrigger value="stats">Stats</TabsTrigger>
            </TabsList>

            <TabsContent value="logs">
              <ContainerLogsProvider>
                <ContainerLogs />
              </ContainerLogsProvider>
            </TabsContent>
            <TabsContent value="inspect">
              <ContainerInspect />
            </TabsContent>
            <TabsContent value="stats">
              <ContainerStatsProvider container={containerInfo}>
                <div className="flex flex-col gap gap-y-4">
                  <MemoryUsage />
                  <CpuUsage />
                  <NetworkUsage />
                </div>
              </ContainerStatsProvider>
            </TabsContent>
          </Tabs>
        </div>
      </div>
    </div>
  );
};
export default ContainerInfoWrapper;
