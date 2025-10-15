import { ImageView } from '@/api/generated/api.types';
import { ImageActionButtons } from '../ImageActionButtons';
import { ImageSateIndicator } from '../ImageStateIndicator';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useNavigate, useParams } from 'react-router';
import { RunImageDialog } from '../dialogs/RunImageDialog';
import { DeleteDialog } from '../delete-dialog';
import { useCallback, useMemo } from 'react';
import { CopyTextToClipboard } from '@/components/ui/CopyTextToClipboard';
import { useRunImageDialog } from '../hooks/useRunImageDialog';
import { useAppContext } from '@/AppContext';
import { ContainerInfoTable } from './ContainerInfoTable';
import { Box, Info, Layers } from 'lucide-react';
import { ImageInfoTable } from './ImageInfoTable';
import { ImageLayerTable } from './ImageLayerTable';
import Loader from '@/components/ui/loader';
import { truncate } from '@/lib/truncate';
import { useDeleteDialog, useRead } from '@/lib/hooks';

const ImageInfoWrapper = () => {
  const navigate = useNavigate();
  const { route } = useAppContext();
  const { platformId, resourceId } = useParams<{ platformId: string; resourceId: string }>();
  const { runDialogData, setRunDialogData } = useRunImageDialog();

  const { setDialogData, dialogData, deleteIsPending, requestDelete } = useDeleteDialog<ImageView>({
    type: 'Image',
    onSuccess: useCallback(() => navigate(`/platforms/${platformId}/images`), [platformId, navigate]),
  });

  const { data, isLoading } = useRead('inspectImage', { platformId, imageId: resourceId });

  const [name, tag] = data?.data.repoTags?.at(0)?.split(':') ?? [];

  const currentTab = useMemo(() => {
    const matches = route?.path.match('[^/]+$');
    const tab = matches && matches[0];
    return tab && ['inspect', 'activity'].includes(tab) ? tab : 'inspect';
  }, [route]);

  const onValueChange = useCallback(
    (tabName: string) => {
      if (resourceId) {
        navigate(`/platforms/${platformId}/images/${resourceId}/${tabName}`);
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
                  <ImageSateIndicator inUse={Object.keys(data?.data.containers ?? {}).length > 0} />
                  <div className="flex flex-col text-md font-bold text-foreground">
                    <span>{truncate(data?.data.repoTags?.at(0) ?? '-:-', 42)}</span>
                    <span className="text-xs text-foreground/40">
                      <CopyTextToClipboard textToCopy={data?.data.id ?? '-'} />
                    </span>
                  </div>
                </div>
                <div className="flex justify-start md:justify-end w-full">
                  <ImageActionButtons
                    selectedImages={[{ id: data?.data.id, imageId: data?.data.id, name, tag } as ImageView]}
                    setDialogData={setDialogData}
                    setRunDialogData={setRunDialogData}
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
                      <ImageInfoTable image={data?.data} />
                    </div>
                  </div>
                  {data?.data.containers.length !== 0 && (
                    <div className="flex flex-col gap-2">
                      <div className="flex flex-row items-center gap-2">
                        <Box width={14} height={14} className="text-muted-foreground" />
                        <div className="text-sm font-semibold text-muted-foreground leading-none">
                          Containers from this image
                        </div>
                      </div>
                      <div className="space-y-1 rounded-sm border p-1 shadow-xs">
                        <ContainerInfoTable image={data?.data} />
                      </div>
                    </div>
                  )}

                  <div className="flex flex-col gap-2">
                    <div className="flex flex-row items-center gap-2">
                      <Layers width={14} height={14} className="text-muted-foreground" />
                      <div className="text-sm font-semibold text-muted-foreground leading-none">Layers</div>
                    </div>
                    <div className="space-y-1 rounded-sm border p-1 shadow-xs">
                      <ImageLayerTable image={data?.data} />
                    </div>
                  </div>
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
      <RunImageDialog runDialogData={runDialogData} setRunDialogData={setRunDialogData} />
    </div>
  );
};

export default ImageInfoWrapper;
