import { useResourceParamType, useLocalStorage, useStickySentinel } from '@/lib/hooks';
import { cn } from '@/lib/utils';
import { useParams } from 'react-router';
import { RequiredDockerInfoComponents } from './types';
import { DockerResourceType } from '@/api/types';
import NotFound from './not-found';
import { CopyToClipboard } from '@/components/custom/copy-to-clipboard';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import Loader from '@/components/ui/loader';
import { AlertMessage } from '@/components/custom/alert-message';
import { ProblemDetails } from '@/api/generated/api.types';
import { DockerResourceInfoComponents } from '@/features';

const DockerResourceInfoPage = () => {
  const type = useResourceParamType()!;

  const Components = DockerResourceInfoComponents[type];
  if (!Components) return <NotFound />;

  return <ResourceInfoView key={type} Components={Components} type={type} />;
};

const ResourceInfoView = <T extends { id: string; name: string }>({ Components, type }: ResourceInfoViewProps<T>) => {
  const { platformId = '', resourceId = '' } = useParams<{
    platformId: string;
    resourceId: string;
  }>();

  const { resource, isLoading, error } = Components.useData(platformId, resourceId);
  const tabs = Components.Tabs ?? [];
  const Header = Components.Header;

  const [activeTab, setActiveTab] = useLocalStorage(
    `${type}-info-${resourceId}.active-tab`,
    tabs[0]?.label ?? 'default',
  );
  const errorDetail = (error as any)?.error as ProblemDetails;
  const { sentinelRef, isStuck } = useStickySentinel(32);

  return (
    <div className="flex flex-col justify-between">
      <div className="px-4 py-4 lg:container sm:px-6 mx-auto">
        <div className="max-w-full rounded-lg border border-border bg-background p-4">
          {isLoading ? (
            <Loader />
          ) : error || !resource ? (
            <AlertMessage title={`${errorDetail?.status}  ${errorDetail?.title}`} type="error">
              {errorDetail?.detail ?? 'Unknown error'}{' '}
            </AlertMessage>
          ) : (
            <div>
              {/* Header */}
              <div className="flex flex-col md:flex-row items-start md:items-center justify-between mb-3">
                <div className="flex items-center gap-1 mb-4 md:mb-0">
                  <Header.Indicator resource={resource} />
                  <div className="flex flex-col text-md font-bold text-foreground">
                    <span>{resource.name}</span>
                    <span className="text-sm text-foreground/40">
                      <CopyToClipboard textToCopy={resource.id ?? '-'} />
                    </span>
                  </div>
                </div>
                <div className="flex justify-start md:justify-end w-full">
                  {Components.Header.ActionButtons && (
                    <div className="flex gap-4 items-center flex-wrap">
                      <Components.Header.ActionButtons resource={resource} />
                    </div>
                  )}
                </div>
              </div>

              {/* Sub Header */}
              {Components.SubHeader && <Components.SubHeader resource={resource} />}

              {/* Tabs */}
              <Tabs value={activeTab} onValueChange={setActiveTab} className="gap-4">
                {/* Sentinel to detect when the tabs reach sticky position */}
                <div ref={sentinelRef} aria-hidden className="h-px" />
                <div
                  className={cn(
                    'sticky top-8 z-30 bg-background left-0 right-0 transition-all duration-200',
                    isStuck ? '-mx-4' : 'mx-0',
                  )}>
                  <TabsList className={cn('w-full overflow-x-auto', isStuck && 'border-b rounded-none py-2')}>
                    {tabs.map((tab) => (
                      <TabsTrigger key={tab.label} value={tab.label}>
                        {tab.label}
                      </TabsTrigger>
                    ))}
                  </TabsList>
                </div>

                {tabs.map((tab) => (
                  <TabsContent key={tab.label} value={tab.label}>
                    <tab.Content resource={resource} />
                  </TabsContent>
                ))}
              </Tabs>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export type ResourceInfoViewProps<T = any> = {
  type: DockerResourceType;
  Components: RequiredDockerInfoComponents<T>;
};

export default DockerResourceInfoPage;
