import { requestErrorStatus } from '@/lib/request-error';
import { PageError } from '@/components/custom/page-error';
import { AppContent } from '@/components/custom/app-content';
import { ResourceReadError } from '@/components/custom/resource-read-error';
import Loader from '@/components/ui/loader';
import { useRead } from '@/lib/hooks';
import type { ResourceType } from '@/api/types';
import type { TabbedResourceComponents, RequiredComponents } from './types';
import { RegularResourceView } from './regular-resource';
import { TabbedResourceView } from './tabbed-resource';

export type ResourceViewProps = {
  Components: RequiredComponents;
  type: ResourceType;
  tab?: ResourceType;
  showTaskSheet?: boolean;
};
export const ResourceView = (props: ResourceViewProps) =>
  props.Components.access === 'administrator' ? (
    <AdministratorResource>
      <ResourceContent {...props} />
    </AdministratorResource>
  ) : (
    <ResourceContent {...props} />
  );

const ResourceContent = ({ Components, type, tab, showTaskSheet = Components.showTaskSheet }: ResourceViewProps) =>
  isTabbedResource(Components) ? (
    <TabbedResourceView key={type} Components={Components} type={type} tab={tab} showTaskSheet={showTaskSheet} />
  ) : (
    <RegularResourceView key={type} Components={Components} type={type} showTaskSheet={showTaskSheet} />
  );

function AdministratorResource({ children }: { children: React.ReactNode }) {
  const profile = useRead('getCurrentProfile');
  if (profile.isPending)
    return (
      <AppContent>
        <Loader />
      </AppContent>
    );
  if (
    requestErrorStatus(profile.error) === 403 ||
    (profile.isSuccess && !profile.data.data.authorization.isAdministrator)
  )
    return <PageError status={403} message="Administrator access is required." />;
  if (profile.isError)
    return (
      <AppContent>
        <ResourceReadError page error={profile.error} refetch={profile.refetch} isFetching={profile.isFetching} />
      </AppContent>
    );
  return children;
}

const isTabbedResource = <T,>(Components: RequiredComponents<T>): Components is TabbedResourceComponents<T> =>
  Array.isArray(Components.Tabs) && Components.Tabs.length > 0;
