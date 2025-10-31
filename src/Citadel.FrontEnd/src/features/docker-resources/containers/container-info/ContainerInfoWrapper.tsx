import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { ContainerLogsProvider } from './logs/ContainerLogsProvider';
import ContainerLogs from './logs/ContainerLogs';
import { useAppContext } from '@/lib/context/app-context';
import { useNavigate } from 'react-router';
import NetworkUsage from './stats/NetworkUsage';
import MemoryUsage from './stats/MemoryUsage';
import CpuUsage from './stats/CpuUsage';
import { useCallback, useEffect, useState } from 'react';
import ContainerInspect from './inspect/ContainerInspect';
import Loader from '@/components/ui/loader';
import { ContainerStatsProvider } from './stats/ContainerStatsProvider';
import { useContainerInfoGroup } from '../hooks/useContainerInfoGroup';
import { ContainerStateStatus, ContainerView } from '@/api/generated/api.types';
import { DeleteDialog } from '../delete-dialog';
import { ActionBarButtons } from '../action-bar-buttons';
import { CopyToClipboard } from '@/components/custom/copy-to-clipboard';
import { fromNow } from '@/lib/dayjs.helper';
import { ContainerInfoTable } from './ContainerInfoTable';
import { useDeleteDialog } from '@/lib/hooks';
import { StateIndicator } from '@/components/custom/state-indicator';

const ContainerInfoWrapper = () => {
  const navigate = useNavigate();
  const { currentContainer, isLoading } = useAppContext();
  const { containerInfo } = useContainerInfoGroup(currentContainer?.containerId, currentContainer?.platformId);
  const { openDialog } = useDeleteDialog<ContainerView>({
    type: 'Container',
    onSuccess: () => navigate(`/platforms/${currentContainer?.platformId}/containers`),
  });

  const [containerId, setContainerId] = useState<string | undefined>();
  const [containerName, setContainerName] = useState<string | undefined>();
  const [containerState, setContainerState] = useState<ContainerStateStatus | undefined>();
  const [statusSnapshot, setStatusSnapshot] = useState<string | undefined>();

  useEffect(() => {
    setContainerName(containerInfo?.name ?? currentContainer?.name);
    setContainerId(containerInfo?.containerId ?? currentContainer?.containerId);
    setContainerState(containerInfo?.state ?? currentContainer?.state);
    if (containerInfo?.state === ContainerStateStatus.Created) {
      setStatusSnapshot(undefined);
    } else if (containerInfo && currentContainer && containerInfo.state === currentContainer.state) {
      const d =
        containerInfo.state === ContainerStateStatus.Running
          ? new Date(currentContainer.startedAt)
          : new Date(currentContainer.finishedAt ?? new Date(Date.now()));
      setStatusSnapshot(fromNow(d));
    } else {
      setStatusSnapshot(fromNow(new Date(Date.now())));
    }
  }, [currentContainer, containerInfo]);

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
        <p className="text-muted-foreground">Nothing to show yet, pick a container to see its details.</p>
      </div>
    );
  }
  return (
    <div className="flex-col justify-between">
      <div className="px-4 py-4 lg:container sm:px-6 mx-auto">
        <div className="flex flex-col gap-4 w-full rounded-lg border-border bg-background p-4">
          {/* Header */}
          <div className="flex flex-col md:flex-row items-start md:items-center justify-between">
            <div className="flex items-center gap-1 mb-4 md:mb-0">
              <StateIndicator value={containerState ?? ContainerStateStatus.Exited} />
              <div className="flex flex-col text-md font-bold text-foreground">
                <span>{containerName?.slice(1)}</span>
                <span className="text-xs text-foreground/40">
                  <CopyToClipboard textToCopy={containerId ?? '-'} />
                </span>
              </div>
            </div>
            <div className="flex justify-start md:justify-end w-full">
              <div className="flex flex-row items-center ">
                <ActionBarButtons
                  selectedRows={[(containerInfo! ?? currentContainer ?? []) as any]}
                  openDialog={openDialog}
                />
              </div>
            </div>
          </div>
          {/* Summary Table */}
          <div className="space-y-1 rounded-sm border p-1 shadow-xs">
            <ContainerInfoTable statusSnapshot={containerState + ' ' + (statusSnapshot ? `(${statusSnapshot})` : '')} />
          </div>
          {/* Tabs */}
          <Tabs value="logs" onValueChange={onValueChange}>
            <TabsList className="w-full">
              <TabsTrigger value="logs">Logs</TabsTrigger>
              <TabsTrigger value="inspect">Inspect</TabsTrigger>
              <TabsTrigger value="stats">Stats</TabsTrigger>
              <TabsTrigger value="activity">Activity</TabsTrigger>
            </TabsList>

            <TabsContent value="logs">
              <ContainerLogsProvider containerId={currentContainer?.containerId}>
                <ContainerLogs />
              </ContainerLogsProvider>
            </TabsContent>
            <TabsContent value="inspect" className="flex flex-col gap-4">
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
            <TabsContent value="activity" className="flex flex-col gap-4"></TabsContent>
          </Tabs>
        </div>
      </div>
      <DeleteDialog />
    </div>
  );
};
export default ContainerInfoWrapper;
