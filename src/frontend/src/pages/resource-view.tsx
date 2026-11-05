import { AppContent } from '@/components/custom/app-content';
import { Button } from '@/components/ui/button';
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
  if (profile.isError)
    return (
      <AppContent>
        <p role="alert">Could not verify administrator access.</p>
        <Button onClick={() => void profile.refetch()}>Retry</Button>
      </AppContent>
    );
  if (!profile.data?.data.authorization.isAdministrator)
    return (
      <AppContent>
        <p role="alert">Administrator access is required.</p>
      </AppContent>
    );
  return children;
}

const isTabbedResource = <T,>(Components: RequiredComponents<T>): Components is TabbedResourceComponents<T> =>
  Array.isArray(Components.Tabs) && Components.Tabs.length > 0;
