import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import ContainerLogsProvider from './logs/ContainerLogsProvider';
import ContainerLogs from './logs/ContainerLogs';
import { Container } from 'lucide-react';
import { useAppContext } from '@/AppProvider';
import { useNavigate } from 'react-router';
import ContainerStatsProvider from './stats/ContainerStatsProvider';
import NetworkUsage from './stats/NetworkUsage';
import MemoryUsage from './stats/MemoryUsage';
import CpuUsage from './stats/CpuUsage';
import { useMemo, useCallback } from 'react';
import ContainerInspect from './inspect/ContainerInspect';
import Loader from '@/components/ui/loader';

const ContainerInfoWrapper = () => {
  const { route, currentContainer, isLoading } = useAppContext();
  const navigate = useNavigate();

  // Memoize the current tab based on the route
  const currentTab = useMemo(() => {
    const matches = route?.path.match('[^/]+$');
    const tab = matches && matches[0];
    return tab && ['logs', 'stats', 'inspect'].includes(tab) ? tab : 'logs';
  }, [route]);

  // Handle tab change
  const onValueChange = useCallback(
    (tabName: string) => {
      if (currentContainer?.id) {
        navigate(`../containers/${currentContainer.containerId.slice(0, 12)}/${tabName}`);
      }
    },
    [navigate, currentContainer],
  );

  if (isLoading) return <Loader />;

  // Render a fallback if currentContainer is undefined
  if (!currentContainer) {
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
            <div className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
              <Container className="h-4 w-4" />
              <span className="sr-only">
                {currentContainer.containerName?.slice(1)} {currentContainer.containerId?.slice(0, 12)}
              </span>
            </div>
            <div className="text-md font-bold text-foreground">
              <span>{currentContainer.containerName?.slice(1)}</span>
              <span className="text-sm text-muted ml-2">({currentContainer.containerId?.slice(0, 12)})</span>
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
              <ContainerStatsProvider>
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
