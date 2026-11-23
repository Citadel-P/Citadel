import { ResourceType as PermissionResource } from '@/api/generated/api.types';
import type { ResourceType } from '@/api/types';
import type { UseQueryResult } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { Link, useParams } from 'react-router';
import { PageError } from '@/components/custom/page-error';
import { AppContent } from '@/components/custom/app-content';
import { ResourceReadError } from '@/components/custom/resource-read-error';
import { Button } from '@/components/ui/button';
import { useRead } from '@/lib/hooks';
import { requestErrorStatus } from '@/lib/request-error';
import NotFound from './not-found';
import { ResourceSkeleton } from './resource-skeleton';

type CreateAccessProps = { type: ResourceType; access?: PermissionResource | 'administrator'; children: ReactNode };

export function ResourceCreateAccess({ platformScoped, ...props }: CreateAccessProps & { platformScoped: boolean }) {
  const { platformId } = useParams();
  if (platformScoped)
    return platformId ? <PlatformAccess id={platformId}>{props.children}</PlatformAccess> : <NotFound />;
  return <ProfileAccess {...props} />;
}

function PlatformAccess({ id, children }: { id: string; children: ReactNode }) {
  const query = useRead('getPlatfom', { id });
  return (
    <AccessResult query={query} allowed={query.data?.data.capabilities?.canWrite}>
      {children}
    </AccessResult>
  );
}

function ProfileAccess({ type, access, children }: CreateAccessProps) {
  const query = useRead('getCurrentProfile');
  const auth = query.data?.data.authorization;
  const permissionType = access === 'administrator' ? type : (access ?? type);
  const hasWrite = auth?.writableResourceTypes?.some((resource) => resource === permissionType);
  const allowed =
    access === 'administrator' ? auth?.isAdministrator && (!(type in PermissionResource) || hasWrite) : hasWrite;
  return (
    <AccessResult query={query} allowed={allowed}>
      {children}
    </AccessResult>
  );
}

function AccessResult({
  query,
  allowed,
  children,
}: {
  query: Pick<UseQueryResult, 'error' | 'isPending' | 'isFetching' | 'fetchStatus' | 'isFetchedAfterMount' | 'refetch'>;
  allowed?: boolean;
  children: ReactNode;
}) {
  if (query.isPending || (query.fetchStatus !== 'idle' && !query.isFetchedAfterMount))
    return <ResourceSkeleton variant="form" />;
  const status = requestErrorStatus(query.error);
  if (status === 404) return <NotFound />;
  if (query.error && status !== 403)
    return (
      <AppContent>
        <ResourceReadError page error={query.error} refetch={query.refetch} isFetching={query.isFetching} />
      </AppContent>
    );
  if (status === 403 || !allowed)
    return (
      <PageError status={403} message="You don’t have permission to create this resource.">
        <Button asChild variant="outline">
          <Link to=".." relative="path">
            Back to list
          </Link>
        </Button>
      </PageError>
    );
  return children;
}
