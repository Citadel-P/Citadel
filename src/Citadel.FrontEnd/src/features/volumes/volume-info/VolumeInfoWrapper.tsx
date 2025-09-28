import { useGETInspect } from '../hooks/useGETInspect';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useNavigate, useParams } from 'react-router';
import { useCallback, useEffect, useMemo } from 'react';
import { useAppContext } from '@/AppContext';
import Loader from '@/components/ui/loader';
import { VolumeStateIndicator } from '../VolumeStateIndicator';
import { useDeleteVolumeDialog } from '../hooks/useDeleteVolumeDialog';
import { DockerVolumeResult } from '@/api/_generated';
import { Box, Info } from 'lucide-react';

import { DeleteVolumeDialog } from '../dialogs/DeleteVolumeDialog';
import { VolumeActionButtons } from '../VolumeActionButtons';
import { ContainerInfoTable } from './ContainerInfoTable';
import { VolumeInfoTable } from './VolumeInfoTable';
import { truncate } from '@/lib/truncate';

const VolumeInfoWrapper = () => {
  const navigate = useNavigate();
  const { route } = useAppContext();
  const { setDialogData, dialogData, deleteIsSuccess, deleteIsPending, requestDelete } = useDeleteVolumeDialog();
  const { platformId, resourceId } = useParams<{ platformId: string; resourceId: string }>();
  const { data, isLoading } = useGETInspect(platformId ?? null, resourceId ?? null);

  useEffect(() => {
    if (deleteIsSuccess) {
      navigate(`/platforms/${platformId}/volumes`);
    }
  }, [deleteIsSuccess, platformId, navigate]);

  // Memoize the current tab based on the route
  const currentTab = useMemo(() => {
    const matches = route?.path.match('[^/]+$');
    const tab = matches && matches[0];
    return tab && ['inspect', 'activity'].includes(tab) ? tab : 'inspect';
  }, [route]);

  // Handle tab change
  const onValueChange = useCallback(
    (tabName: string) => {
      if (resourceId) {
        navigate(`/platforms/${platformId}/volumes/${resourceId}/${tabName}`);
      }
    },
    [navigate, resourceId, platformId],
  );

  return (
    <div className="flex-col justify-between">
      <div className="px-4 py-4 lg:container sm:px-6 mx-auto">
        <div className="max-w-full rounded-lg border-border bg-background p-4">
          {isLoading ? (
            <Loader />
          ) : (
            <div>
              <div className="flex flex-col md:flex-row items-start md:items-center justify-between mb-3">
                <div className="flex items-center gap-1 mb-4 md:mb-0">
                  <VolumeStateIndicator inUse={data?.data.inUse ?? false} />
                  <div className="flex text-wrap text-md font-bold text-foreground">
                    <span>{truncate(data?.data.id ?? '', 42)}</span>
                  </div>
                </div>
                <div className="flex justify-start md:justify-end ">
                  <VolumeActionButtons
                    selectedVolumes={[
                      {
                        id: data?.data.id,
                        inUse: data?.data?.containers?.length !== 0,
                      } as DockerVolumeResult,
                    ]}
                    setDialogData={setDialogData}
                    showInspectButton={false}
                  />
                </div>
              </div>

              <Tabs value={currentTab} onValueChange={onValueChange} className="gap-4">
                <TabsList className="w-full">
                  <TabsTrigger value="inspect">Inspect</TabsTrigger>
                  <TabsTrigger value="activity">Activity</TabsTrigger>
                </TabsList>

                <TabsContent value="inspect" className="flex flex-col gap-8">
                  <div className="flex flex-col gap-2">
                    <div className="flex flex-row items-center gap-2">
                      <Info width={14} height={14} className="text-muted-foreground" />
                      <div className="text-sm font-semibold text-muted-foreground leading-none">Details</div>
                    </div>
                    <div className="space-y-1 rounded-sm border p-1 shadow-xs">
                      <VolumeInfoTable volume={data?.data} />
                    </div>
                  </div>
                  {Object.keys(data?.data?.containers ?? {}).length !== 0 && (
                    <div className="flex flex-col gap-2">
                      <div className="flex flex-row items-center gap-2">
                        <Box width={14} height={14} className="text-muted-foreground" />
                        <div className="text-sm font-semibold text-muted-foreground leading-none">
                          Containers using this volume
                        </div>
                      </div>
                      <div className="space-y-1 rounded-sm border p-1 shadow-xs">
                        <ContainerInfoTable volume={data?.data} />
                      </div>
                    </div>
                  )}
                </TabsContent>
                <TabsContent value="activity">activity</TabsContent>
              </Tabs>
            </div>
          )}
        </div>
      </div>
      <DeleteVolumeDialog
        dialogData={dialogData}
        setDialogData={setDialogData}
        requestDelete={requestDelete}
        deleteIsPending={deleteIsPending}
      />
    </div>
  );
};

export default VolumeInfoWrapper;
