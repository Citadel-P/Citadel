import { Images as LucidImages } from 'lucide-react';
import { useContextSelector } from 'use-context-selector';
import { AlertMessage } from '@/components/ui/alert-message';
import { ImagesContext } from './ImagesProvider';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router';
import { AppContext } from '@/AppProvider';
import ExternalImages from './ExternalImages';
import LocalImagesTable from './LocalImagesTable';
import { ActionBar } from './ActionBar';
const Images = () => {
  const navigate = useNavigate();
  const route = useContextSelector(AppContext, (v) => v?.route);
  const currentPlatform = useContextSelector(AppContext, (v) => v?.currentPlatform);
  const platformOnline = useContextSelector(ImagesContext, (v) => v?.isPlatformOnline);
  const [currentTab, setCurrentTab] = useState<string>();

  useEffect(() => {
    const matches = route?.path.match('[^/]+$');
    const tab = matches && matches[0];
    const _ = tab && ['local', 'external'].includes(tab) ? setCurrentTab(tab) : setCurrentTab('local');
  }, [setCurrentTab, route]);

  const onValueChange = (tabName: string) => {
    navigate(`../platforms/${currentPlatform?.id}/images/${tabName}`);
  };

  return (
    <div className="flex-col justify-between min-h-[calc(100%-4rem)] relative">
      <div className="px-4 py-4 lg:container sm:px-6 mx-auto">
        <div className="w-full rounded-lg border-border bg-background p-4">
          <div className="sm:flex sm:justify-between">
            <div className="mb-3 flex items-baseline gap-1">
              <div className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                <LucidImages className="h-4 w-4" />
                <span className="sr-only">Images</span>
              </div>
              <div className="text-md font-bold text-foreground">Images</div>
            </div>
          </div>
          {!platformOnline && (
            <AlertMessage type="warning" hasTitle={true}>
              This platform is not connected, please try to update or reconnect the platform.{' '}
            </AlertMessage>
          )}
          <Tabs value={currentTab} onValueChange={onValueChange}>
            <TabsList className="w-full justify-start bg-muted/20 rounded-sm">
              <TabsTrigger value="local">Local</TabsTrigger>
              <TabsTrigger value="external">External</TabsTrigger>
            </TabsList>

            <TabsContent value="local">
              <LocalImagesTable platfomrId={currentPlatform?.id} />
            </TabsContent>
            <TabsContent value="external">
              <div className="flex flex-col gap-3">
                <ExternalImages />
              </div>
            </TabsContent>
          </Tabs>
        </div>
      </div>
      <ActionBar />
    </div>
  );
};

export default Images;
