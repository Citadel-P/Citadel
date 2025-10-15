import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useNavigate, useParams } from 'react-router';
import { useCallback, useMemo } from 'react';
import { CopyTextToClipboard } from '@/components/ui/CopyTextToClipboard';
import { useAppContext } from '@/AppContext';
import Loader from '@/components/ui/loader';
import { NetworkStateIndicator } from '../NetworkSateIndicator';
import { NetworkActionButtons } from '../NetworkActionButtons';
import { DockerNetworkResult } from '@/api/generated/api.types';
import { DeleteDialog } from '../delete-dialog';
import { Box, Info, Share2 } from 'lucide-react';
import { ContainerInfoTable } from './ContainerInfoTable';
import { NetworkInfoTable } from './NetworkInfoTable';
import { IPAMInfoTable } from './IPAMInfoTable';
import { useDeleteDialog, useRead } from '@/lib/hooks';

const NetworkInfoWrapper = () => {
  const navigate = useNavigate();
  const { route } = useAppContext();

  const { platformId, resourceId } = useParams<{ platformId: string; resourceId: string }>();
  const { data, isLoading } = useRead('inspectNetwork', { platformId, networkId: resourceId });

  const { setDialogData, dialogData, deleteIsPending, requestDelete } = useDeleteDialog<DockerNetworkResult>({
    type: 'Network',
    onSuccess: useCallback(() => navigate(`/platforms/${platformId}/networks`), [platformId, navigate]),
  });

  const currentTab = useMemo(() => {
    const matches = route?.path.match('[^/]+$');
    const tab = matches && matches[0];
    return tab && ['inspect', 'activity'].includes(tab) ? tab : 'inspect';
  }, [route]);

  // Handle tab change
  const onValueChange = useCallback(
    (tabName: string) => {
      if (resourceId) {
        navigate(`/platforms/${platformId}/networks/${resourceId}/${tabName}`);
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
                  <NetworkStateIndicator inUse={Object.keys(data?.data.containers ?? {}).length > 0} />
                  <div className="flex flex-col text-md font-bold text-foreground">
                    <span>{data?.data.name}</span>
                    <span className="text-xs text-foreground/40">
                      <CopyTextToClipboard textToCopy={data?.data.id ?? '-'} />
                    </span>
                  </div>
                </div>
                <div className="flex justify-start md:justify-end w-full">
                  <NetworkActionButtons
                    selectedNetworks={[
                      {
                        id: data?.data.id,
                        name: data?.data.name,
                        inUse: Object.keys(data?.data.containers ?? {}).length > 0,
                      } as DockerNetworkResult,
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
                      <NetworkInfoTable network={data?.data} />
                    </div>
                  </div>
                  {Object.keys(data?.data?.containers ?? {}).length !== 0 && (
                    <div className="flex flex-col gap-2">
                      <div className="flex flex-row items-center gap-2">
                        <Box width={14} height={14} className="text-muted-foreground" />
                        <div className="text-sm font-semibold text-muted-foreground leading-none">
                          Containers in this network
                        </div>
                      </div>
                      <div className="space-y-1 rounded-sm border p-1 shadow-xs">
                        <ContainerInfoTable network={data?.data} />
                      </div>
                    </div>
                  )}

                  {data?.data?.ipam?.config?.length !== 0 && (
                    <div className="flex flex-col gap-2">
                      <div className="flex flex-row items-center gap-2">
                        <Share2 width={14} height={14} className="text-muted-foreground" />
                        <div className="text-sm font-semibold text-muted-foreground leading-none">IPAM</div>
                      </div>
                      <div className="space-y-1 rounded-sm border p-1 shadow-xs">
                        <IPAMInfoTable ipam={data?.data?.ipam ?? undefined} />
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
      <DeleteDialog
        dialogData={dialogData}
        setDialogData={setDialogData}
        requestDelete={requestDelete}
        deleteIsPending={deleteIsPending}
      />
    </div>
  );
};

export default NetworkInfoWrapper;
