import { ImageView } from '@/api/_generated';
import { useGETInspect } from '../hooks/useGETInspect';
import { ImageActionButtons } from '../ImageActionButtons';
import { ImageSateIndicator } from '../ImageStateIndicator';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useNavigate, useParams } from 'react-router';
import { RunImageDialog } from '../dialogs/RunImageDialog';
import { DeleteLocalImageDialog } from '../dialogs/DeleteLocalImageDialog';
import { useDeleteImageDialog } from '../hooks/useDeleteImageDialog';
import { useEffect } from 'react';
import { CopyTextToClipboard } from '@/components/ui/CopyTextToClipboard';

const ImageInfoWrapper = () => {
  const navigate = useNavigate();
  const { platformId, imageId } = useParams<{ platformId: string; imageId: string }>();
  const { setDialogData, dialogData, deleteIsSuccess, deleteIsPending, requestDelete } = useDeleteImageDialog();
  const { data } = useGETInspect(platformId ?? null, imageId ?? null);

  useEffect(() => {
    if (deleteIsSuccess) {
      navigate(`/platforms/${platformId}/images`);
    }
  }, [deleteIsSuccess, platformId, navigate]);

  return (
    <div className="flex-col justify-between">
      <div className="px-4 py-4 lg:container sm:px-6 mx-auto">
        <div className="max-w-full rounded-lg border-border bg-background p-4">
          {/* Header */}
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-1">
              <ImageSateIndicator inUse={Object.keys(data?.data.containers ?? {}).length > 0} />
              <div className="flex flex-col  gap-1 text-md font-bold text-foreground">
                <span>{data?.data.repoTags?.at(0) ?? '-:-'}</span>
                <span className="text-xs text-foreground/40">
                  <CopyTextToClipboard textToCopy={data?.data.id ?? '-'} />
                </span>
              </div>
            </div>
            <div className="flex justify-end">
              <ImageActionButtons
                selectedImages={[{ id: data?.data.id } as ImageView]}
                setDialogData={setDialogData}
                setRunDialogData={() => {}}
              />
            </div>
          </div>

          {/* Tabs */}
          <Tabs>
            <TabsList className="w-full justify-start bg-muted/20 rounded-sm">
              <TabsTrigger value="logs">Logs</TabsTrigger>
              <TabsTrigger value="inspect">Inspect</TabsTrigger>
              <TabsTrigger value="stats">Stats</TabsTrigger>
            </TabsList>

            <TabsContent value="logs">logs</TabsContent>
            <TabsContent value="inspect">inspect</TabsContent>
            <TabsContent value="stats">stats</TabsContent>
          </Tabs>
        </div>
      </div>
      <DeleteLocalImageDialog
        dialogData={dialogData}
        setDialogData={setDialogData}
        requestDelete={requestDelete}
        deleteIsPending={deleteIsPending}
      />
    </div>
  );
};

export default ImageInfoWrapper;
