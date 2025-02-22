import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import ContainerLogsProvider from './logs/ContainerLogsProvider';
import ContainerLogs from './logs/ContainerLogs';
import { Container } from 'lucide-react';
import { AppContext } from '@/AppProvider';
import { useContextSelector } from 'use-context-selector';
import { useNavigate } from 'react-router';
import ContainerStatsProvider from './stats/ContainerStatsProvider';
import NetworkUsage from './stats/NetworkUsage';
import MemoryUsage from './stats/MemoryUsage';
import CpuUsage from './stats/CpuUsage';
import { useEffect, useState } from 'react';

const ContainerInfoWrapper = () => {
  const route = useContextSelector(AppContext, (v) => v?.route);
  const currentContainer = useContextSelector(AppContext, (v) => v?.currentContainer);
  const navigate = useNavigate();
  const [currentTab, setCurrentTab] = useState<string>();

  useEffect(() => {
    const tab = route?.path.match('[^/]+$')![0];
    tab && ['logs', 'stats'].includes(tab) ? setCurrentTab(tab) : setCurrentTab('logs');
  }, [setCurrentTab, route]);

  const onValueChange = (tabName: string) => {
    navigate(`../containers/${currentContainer?.containerId?.slice(0, 12)}/${tabName}`);
  };

  return (
    <div className="min-h-[calc(100%-2rem)] relative">
      <div className="px-4 py-4 lg:container sm:px-6 mx-auto">
        <div className="max-w-full rounded-lg border-border bg-background p-4">
          <div className="flex items-baseline gap-1 mb-3">
            <div className="inline-flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
              <Container className="h-4 w-4" />
              <span className="sr-only">
                {currentContainer?.name?.slice(1)} {currentContainer?.containerId?.slice(0, 12)}
              </span>
            </div>
            <div className="text-md font-bold text-foreground">
              <span>{currentContainer?.name?.slice(1)}</span>
              <span className="text-sm text-muted ml-2">({currentContainer?.containerId?.slice(0, 12)})</span>
            </div>
          </div>
          <Tabs value={currentTab} onValueChange={onValueChange}>
            <TabsList className="w-full justify-start bg-muted/20 rounded-sm">
              <TabsTrigger value="logs">Logs</TabsTrigger>
              <TabsTrigger value="stats">Stats</TabsTrigger>
            </TabsList>
            <TabsContent value="logs">
              <ContainerLogsProvider>
                <ContainerLogs />
              </ContainerLogsProvider>
            </TabsContent>
            <TabsContent value="stats">
              <ContainerStatsProvider>
                <div className="flex flex-col gap gap-y-3">
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
