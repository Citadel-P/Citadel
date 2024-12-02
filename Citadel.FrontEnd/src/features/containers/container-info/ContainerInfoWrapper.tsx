import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import ContainerLogsProvider from './logs/ContainerLogsProvider';
import ContainerLogs from './logs/ContainerLogs';
import { Container } from 'lucide-react';
import { useAppContext } from '@/AppProvider';
import { useNavigate } from 'react-router-dom';
import ContainerStats from './stats/ContainerStats';
import ContainerStatsProvider from './stats/ContainerStatsProvider';

const ContainerInfoWrapper = () => {
  const { route, currentContainer } = useAppContext();
  const navigate = useNavigate();

  const handleClick = (fragment: string) =>
    navigate(`../containers/${currentContainer?.containerId?.slice(0, 12)}/${fragment}`);

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
          <Tabs defaultValue={route.path.endsWith('logs') ? 'logs' : 'stats'}>
            <TabsList className="w-full justify-start bg-muted/20 rounded-sm">
              <TabsTrigger value="logs" onClick={() => handleClick('logs')}>
                Logs
              </TabsTrigger>
              <TabsTrigger value="stats" onClick={() => handleClick('stats')}>
                Stats
              </TabsTrigger>
            </TabsList>
            <TabsContent value="logs">
              <ContainerLogsProvider>
                <ContainerLogs />
              </ContainerLogsProvider>
            </TabsContent>
            <TabsContent value="stats">
              <ContainerStatsProvider>
                <ContainerStats />
              </ContainerStatsProvider>
            </TabsContent>
          </Tabs>
        </div>
      </div>
    </div>
  );
};

export default ContainerInfoWrapper;
