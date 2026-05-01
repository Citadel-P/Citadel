import { ResourceComponents } from '@/features';
import { useResourceParamType } from '@/lib/hooks';
import { TabbedResourceComponents, RequiredComponents } from './types';
import NotFound from './not-found';
import { RegularResourceView } from './regular-resource';
import { TabbedResourceView } from './tabbed-resource';

const ResourcePage = () => {
  const { type, tab } = useResourceParamType();
  const Components = ResourceComponents[type];

  if (!Components) return <NotFound />;

  if (isTabbedResource(Components)) {
    return <TabbedResourceView key={type} Components={Components} type={type} tab={tab} />;
  }

  return <RegularResourceView key={type} Components={Components} type={type} />;
};

const isTabbedResource = <T,>(Components: RequiredComponents<T>): Components is TabbedResourceComponents<T> =>
  Array.isArray(Components.Tabs) && Components.Tabs.length > 0;

export default ResourcePage;
