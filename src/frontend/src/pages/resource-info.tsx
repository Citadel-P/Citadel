import { canShowCachedResource } from '@/lib/request-error';
import { AppContent } from '@/components/custom/app-content';
import { ResourceReadError } from '@/components/custom/resource-read-error';
import { CopyToClipboard } from '@/components/custom/copy-to-clipboard';
import { ResourceTabs } from '@/components/custom/resource-tabs';
import { ResourceSkeleton } from './resource-skeleton';
import { useParams } from 'react-router';
import { RequiredDockerInfoComponents } from './types';
import { Box } from 'lucide-react';

export const ResourceInfoView = <T extends { id: string; name: string }>({
  Components,
  type,
  showHeaderId = true,
}: ResourceInfoViewProps<T>) => {
  const { platformId = '', resourceId = '' } = useParams<{
    platformId: string;
    resourceId: string;
  }>();
  const { resource, isLoading, error, refetch, isFetching } = Components.useData(platformId, resourceId);
  if (error && !canShowCachedResource(error))
    return (
      <AppContent>
        <ResourceReadError error={error} refetch={refetch} isFetching={isFetching} />
      </AppContent>
    );
  const Header = Components.Header;
  const Icon = Header.Icon ?? Box;

  return (
    <div className="flex-col justify-between relative">
      <AppContent>
        <div className="flex flex-col gap-(--section-gap)">
          {!!error && resource && <ResourceReadError error={error} refetch={refetch} isFetching={isFetching} stale />}
          {isLoading && !resource ? (
            <ResourceSkeleton variant="detail" embedded />
          ) : !resource ? (
            <ResourceReadError error={error ?? { status: 404 }} refetch={refetch} isFetching={isFetching} />
          ) : (
            resource && (
              <>
                <header className="flex min-w-0 flex-col overflow-hidden rounded-lg border bg-card shadow-xs lg:flex-row lg:items-center">
                  <div className="flex min-w-0 flex-1 items-start gap-4 p-(--surface-padding)">
                    <span
                      aria-hidden="true"
                      className="flex size-12 shrink-0 items-center justify-center rounded-lg border border-primary/15 bg-primary/5 text-primary dark:text-foreground">
                      <Icon className="size-6" />
                    </span>
                    <div className="min-w-0 flex-1">
                      <div className="flex min-w-0 flex-wrap items-center gap-2">
                        <h1 className="min-w-0 max-w-full text-2xl font-semibold tracking-tight wrap-anywhere">
                          {resource.name}
                        </h1>
                        {Header.Status ? (
                          <Header.Status resource={resource} />
                        ) : Header.Indicator ? (
                            <Header.Indicator resource={resource} />                       
                        ) : null}
                        {Header.NameSuffix && <Header.NameSuffix resource={resource} />}
                      </div>
                      {showHeaderId && resource.id !== resource.name && (
                        <div className="mt-2 flex min-w-0 items-center gap-2 text-xs text-muted-foreground">
                          <span className="shrink-0">ID</span>
                          <CopyToClipboard
                            textToCopy={resource.id ?? '-'}
                            textClassName="font-mono"
                            groupClassName="[&_button]:visible [&_svg]:text-muted-foreground"
                          />
                        </div>
                      )}
                    </div>
                  </div>
                  <div
                    role="group"
                    aria-label={`${type} actions`}
                    className="flex min-w-0 flex-wrap items-center gap-3 border-t bg-muted/10 px-(--surface-padding) py-3 lg:max-w-[55%] lg:shrink-0 lg:justify-end lg:border-t-0 lg:bg-transparent lg:py-(--surface-padding)">
                    <Header.ActionButtons resource={resource} />
                  </div>
                </header>
                {Components.SubHeader && <Components.SubHeader resource={resource} />}
                <ResourceTabs
                  localKey={`${type}-info-${resourceId}`}
                  resource={resource as any}
                  tabs={Components.Tabs}
                />
                {Components.Footer && <Components.Footer resource={resource} />}
              </>
            )
          )}
        </div>
      </AppContent>
    </div>
  );
};

export type ResourceInfoViewProps<T = any> = {
  type: string;
  showHeaderId?: boolean;
  Components: RequiredDockerInfoComponents<T>;
};
