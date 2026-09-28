import { Suspense } from 'react';
import { useParams } from 'react-router';
import { ResourceFormPages } from '@/features';
import { useResourceParamType } from '@/lib/hooks';
import { ResourceSkeleton } from './resource-skeleton';
import NotFound from './not-found';

export const ResourceForm = ({ mode }: { mode: 'add' | 'edit' }) => {
  const { type, tab } = useResourceParamType();
  const { platformId } = useParams();
  const routeType = tab ?? type;
  const resolvedType = platformId && routeType === 'Service' ? 'SwarmService' : routeType;
  const Page = ResourceFormPages[resolvedType];
  if (!Page) return <NotFound />;
  return (
    <Suspense fallback={<ResourceSkeleton variant={'form'} />}>
      <Page key={resolvedType} mode={mode} type={resolvedType} />
    </Suspense>
  );
};
