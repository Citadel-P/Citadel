import { useResourceParamType } from '@/lib/hooks';
import { useParams } from 'react-router';
import { RequiredDockerInfoComponents } from './types';
import { DockerResourceType } from '@/api/types';
import NotFound from './not-found';
import { CopyToClipboard } from '@/components/custom/copy-to-clipboard';
import Loader from '@/components/ui/loader';
import { AlertMessage } from '@/components/custom/alert-message';
import { ProblemDetails } from '@/api/generated/api.types';
import { DockerResourceInfoComponents } from '@/features';
import { ResourceTabs } from '@/components/custom/resource-tabs';

const ResourceDockerInfoPage = () => {
  const { type } = useResourceParamType()!;

  const Components = DockerResourceInfoComponents[type as DockerResourceType];
  if (!Components) return <NotFound />;

  return <ResourceInfoView key={type} Components={Components} type={type as DockerResourceType} />;
};

const ResourceInfoView = <T extends { id: string; name: string }>({ Components, type }: ResourceInfoViewProps<T>) => {
  const { platformId = '', resourceId = '' } = useParams<{
    platformId: string;
    resourceId: string;
  }>();

  const { resource, isLoading, error } = Components.useData(platformId, resourceId);

  const tabs = Components.Tabs ?? [];
  const Header = Components.Header;

  const key = `${type}-info-${resourceId}`;
  const errorDetail = (error as any)?.error as ProblemDetails;

  return (
    <div className="flex-col justify-between relative">
      <div className="mx-auto w-full max-w-[1440px] px-4 py-4 sm:px-6">
        <div className="max-w-full rounded-lg border border-border bg-background p-4">
          <div className="flex flex-col gap-2">
            {(isLoading || !resource) && !error ? (
              <Loader />
            ) : error ? (
              <AlertMessage title={`${errorDetail?.status}  ${errorDetail?.title}`} type="error">
                {errorDetail?.detail ?? 'Unknown error'}{' '}
              </AlertMessage>
            ) : (
              resource && (
                <>
                  {/* Header */}
                  <div className="flex flex-col sm:flex-row gap-2 items-start justify-between">
                    <div className="flex items-center sm:gap-2">
                      <Header.Indicator resource={resource} />
                      <div className="flex flex-col text-md font-bold text-foreground min-w-0">
                        <span className="inline-flex min-w-0 items-center gap-1.5">
                          <span className="truncate">{resource.name}</span>
                          {Header.NameSuffix && <Header.NameSuffix resource={resource} />}
                        </span>

                        <span className="text-sm text-foreground/40 min-w-0 max-w-50 xl:max-w-full">
                          <CopyToClipboard textToCopy={resource.id ?? '-'} />
                        </span>
                      </div>
                    </div>
                    {Components.Header.ActionButtons && (
                      <div className="flex gap-4 items-center overflow-auto flex-wrap">
                        <Components.Header.ActionButtons resource={resource} />
                      </div>
                    )}
                  </div>
                  {/* Sub Header */}
                  {Components.SubHeader && <Components.SubHeader resource={resource} />}
                  {/* Tabs */}
                  <ResourceTabs localKey={key} resource={resource as any} tabs={tabs} />
                </>
              )
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

export type ResourceInfoViewProps<T = any> = {
  type: DockerResourceType;
  Components: RequiredDockerInfoComponents<T>;
};

export default ResourceDockerInfoPage;
