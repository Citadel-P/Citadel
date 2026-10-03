import { Suspense } from 'react';
import { ResourcePages } from '@/features';
import { useResourceParamType } from '@/lib/hooks';
import { ResourceSkeleton } from './resource-skeleton';
import NotFound from './not-found';

const ResourcePage = () => {
  const { type, tab } = useResourceParamType();
  const Page = ResourcePages[type];
  if (!Page) return <NotFound />;
  return (
    <Suspense fallback={<ResourceSkeleton variant={type === 'Platform' ? 'platform' : 'list'} />}>
      <Page key={type} type={type} tab={tab} />
    </Suspense>
  );
};
export default ResourcePage;
