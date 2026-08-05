import { ProblemDetails } from '@/api/generated/api.types';
import { AlertMessage } from '@/components/custom/alert-message';
import { PageContainer } from '@/components/custom/common';
import { CopyToClipboard } from '@/components/custom/copy-to-clipboard';
import { ResourceTabs } from '@/components/custom/resource-tabs';
import Loader from '@/components/ui/loader';
import { useParams } from 'react-router';
import { RequiredDockerInfoComponents } from './types';

export const ResourceInfoView = <T extends { id: string; name: string }>({
  Components,
  type,
}: ResourceInfoViewProps<T>) => {
  const { platformId = '', resourceId = '' } = useParams<{
    platformId: string;
    resourceId: string;
  }>();
  const { resource, isLoading, error } = Components.useData(platformId, resourceId);
  const Header = Components.Header;
  const errorDetail = (error as { error?: ProblemDetails } | null)?.error;

  return (
    <div className="flex-col justify-between relative">
      <PageContainer className="max-w-full border">
        <div className="flex flex-col gap-2">
          {(isLoading || !resource) && !error ? (
            <Loader />
          ) : error ? (
            <AlertMessage title={`${errorDetail?.status ?? ''} ${errorDetail?.title ?? 'Error'}`.trim()} type="error">
              {errorDetail?.detail ?? 'Unknown error'}
            </AlertMessage>
          ) : (
            resource && (
              <>
                <div className="flex flex-col sm:flex-row gap-2 items-start justify-between">
                  <div className="flex items-center sm:gap-2">
                    {Header.Indicator && <Header.Indicator resource={resource} />}
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
                  <div className="flex w-full min-w-0 flex-wrap items-center gap-4 sm:w-auto">
                    <Header.ActionButtons resource={resource} />
                  </div>
                </div>
                {Components.SubHeader && <Components.SubHeader resource={resource} />}
                <ResourceTabs
                  localKey={`${type}-info-${resourceId}`}
                  resource={resource as any}
                  tabs={Components.Tabs}
                />
              </>
            )
          )}
        </div>
      </PageContainer>
    </div>
  );
};

export type ResourceInfoViewProps<T = any> = {
  type: string;
  Components: RequiredDockerInfoComponents<T>;
};
