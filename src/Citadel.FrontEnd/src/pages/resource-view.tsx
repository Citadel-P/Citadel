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
export const ResourceView = ({ Components, type, tab, showTaskSheet }: ResourceViewProps) =>
  isTabbedResource(Components) ? (
    <TabbedResourceView key={type} Components={Components} type={type} tab={tab} />
  ) : (
    <RegularResourceView key={type} Components={Components} type={type} showTaskSheet={showTaskSheet} />
  );

const isTabbedResource = <T,>(Components: RequiredComponents<T>): Components is TabbedResourceComponents<T> =>
  Array.isArray(Components.Tabs) && Components.Tabs.length > 0;
