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
import { useContainerInfoGroup } from '../hooks/useContainerInfoGroup';
import { ContainerStateStatus, ContainerView } from '@/api/generated/api.types';
import { ContainerStateIndicator } from '../ContainerStateIndicator';
import { DeleteDialog } from '../delete-dialog';
import { ActionBarButtons } from '../action-bar-buttons';
import { CopyTextToClipboard } from '@/components/ui/CopyTextToClipboard';
import { fromNow } from '@/lib/dayjs.helper';
import { ContainerInfoTable } from './ContainerInfoTable';
import { useDeleteDialog } from '@/lib/hooks';
import { DockerContainerView } from '@/api/types';

const ContainerInfoWrapper = () => {
  const navigate = useNavigate();
  const { route, currentContainer, isLoading } = useAppContext();
  const { containerInfo } = useContainerInfoGroup(currentContainer?.containerId, currentContainer?.platformId);
  const {
    deleteIsPending: isPending,
    requestDelete,
    dialogData,
    setDialogData,
  } = useDeleteDialog<ContainerView | DockerContainerView>({
    type: 'Container',
    onSuccess: useCallback(
      () => navigate(`/platforms/${currentContainer?.platformId}/containers`),
      [currentContainer?.platformId, navigate],
    ),
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

  // Memoize the current tab based on the route
  const currentTab = useMemo(() => {
    const matches = route?.path.match('[^/]+$');
    const tab = matches && matches[0];
    return tab && ['logs', 'stats', 'inspect', 'activity'].includes(tab) ? tab : 'logs';
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
              <ContainerStateIndicator stat={containerState ?? ContainerStateStatus.Exited} />
              <div className="flex flex-col text-md font-bold text-foreground">
                <span>{containerName?.slice(1)}</span>
                <span className="text-xs text-foreground/40">
                  <CopyTextToClipboard textToCopy={containerId ?? '-'} />
                </span>
              </div>
            </div>
            <div className="flex justify-start md:justify-end w-full">
              <div className="flex flex-row items-center ">
                <ActionBarButtons
                  selectedContainers={[containerInfo! ?? currentContainer]}
                  setDialogData={setDialogData}
                />
              </div>
            </div>
          </div>
          {/* Summary Table */}
          <div className="space-y-1 rounded-sm border p-1 shadow-xs">
            <ContainerInfoTable statusSnapshot={containerState + ' ' + (statusSnapshot ? `(${statusSnapshot})` : '')} />
          </div>
          {/* Tabs */}
          <Tabs value={currentTab} onValueChange={onValueChange}>
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
      <DeleteDialog
        requestDelete={requestDelete}
        isPending={isPending}
        dialogData={dialogData}
        setDialogData={setDialogData}
      />
    </div>
  );
};
export default ContainerInfoWrapper;
