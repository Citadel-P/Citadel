import { Suspense } from 'react';
import { ResourceSkeleton } from './resource-skeleton';
import { SwarmResourcePathMap } from '@/api/types';
import { SwarmResourcePages, SwarmResourceInfoPages } from '@/features';
import NotFound from './not-found';
import { useParams } from 'react-router';

const ResourceSwarmPage = () => {
  const { type = '', resourceId } = useParams<{ type: string; resourceId?: string }>();
  const swarmType = SwarmResourcePathMap[type as keyof typeof SwarmResourcePathMap];
  if (!swarmType) return <NotFound />;

  const Page = resourceId ? SwarmResourceInfoPages[swarmType] : SwarmResourcePages[swarmType];
  return (
    <Suspense fallback={<ResourceSkeleton variant={resourceId ? 'detail' : 'list'} />}>
      <Page key={`${swarmType}-${resourceId ?? 'list'}`} type={swarmType} showTaskSheet={false} />
    </Suspense>
  );
};
export default ResourceSwarmPage;
