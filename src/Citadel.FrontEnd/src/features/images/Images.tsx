import { HardDrive } from 'lucide-react';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useMemo, useCallback } from 'react';
import { useNavigate } from 'react-router';
import { useAppContext } from '@/AppContext';
import ExternalRepositories from './ExternalRepositories';
import LocalImagesTable from './LocalImagesTable';
import Loader from '@/components/ui/loader';
import { SearchField } from '@/components/ui/SearchField';
import { useImagesContext } from './ImagesContext';
import { ImageActionBar } from './ImageActionBar';

const Images = () => {
  const navigate = useNavigate();
  const { isLoading, currentPlatform, route } = useAppContext();
  const { onSearch } = useImagesContext();

  // Memoize the current tab based on the route
  const currentTab = useMemo(() => {
    const matches = route?.path.match('[^/]+$');
    const tab = matches && matches[0];
    return tab && ['local', 'external'].includes(tab) ? tab : 'local';
  }, [route]);

  // Handle tab change
  const onValueChange = useCallback(
    (tabName: string) => {
      if (currentPlatform?.id) {
        navigate(`../platforms/${currentPlatform.id}/images/${tabName}`);
      }
    },
    [navigate, currentPlatform],
  );

  if (isLoading) return <Loader />;

  // Render a fallback if currentPlatform is undefined
  if (!currentPlatform) {
    return (
      <div className="flex justify-center items-center h-full">
        <p className="text-muted-foreground">No platform selected. Please select a platform to view images.</p>
      </div>
    );
  }

  return (
    <div className="flex-col justify-between relative">
      <div className="px-4 py-4 lg:container sm:px-6 mx-auto">
        <div className="w-full rounded-lg border-border bg-background p-4">
          {/* Header */}
          <div className="sm:flex sm:justify-between">
            <div className="mb-3 flex items-baseline gap-1">
              <div className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                <HardDrive className="h-4 w-4" />
                <span className="sr-only">Images</span>
              </div>
              <div className="text-md font-bold text-foreground">Images</div>
            </div>
            {currentTab == 'local' && <SearchField onSearch={onSearch} />}
          </div>
          {/* Tabs */}
          <Tabs value={currentTab} onValueChange={onValueChange}>
            <TabsList className="w-full">
              <TabsTrigger value="local">Local</TabsTrigger>
              <TabsTrigger value="external">External</TabsTrigger>
            </TabsList>

            <TabsContent value="local">
              <div className="space-y-1 rounded-sm border p-1 shadow-xs">
                <LocalImagesTable />
              </div>
            </TabsContent>
            <TabsContent value="external">
              <div className="flex flex-col gap-3">
                <ExternalRepositories />
              </div>
            </TabsContent>
          </Tabs>
        </div>
      </div>
      <ImageActionBar />
    </div>
  );
};

export default Images;
