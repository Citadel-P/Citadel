import { ServerCog } from 'lucide-react';
import type { TabbedResourceComponents } from '@/pages/types';
import { InstanceStatus } from './status';
import { InstanceRestartButton } from './restart';
import { InstanceActivities } from './activities';
import { InstanceSettingsForm } from './settings';

export const InstanceComponents: TabbedResourceComponents = {
  Icon: ServerCog,
  access: 'administrator',
  showTaskSheet: true,
  tabsVariant: 'bordered',
  SubHeader: InstanceStatus,
  header: {
    subtitle: 'Configure security, monitoring and Core execution defaults.',
    variant: 'plain',
    showSearch: false,
    showAdd: false,
    Actions: InstanceRestartButton,
  },
  Tabs: [
    { label: 'Config', slug: 'config', Content: InstanceSettingsForm },
    { label: 'Activities', slug: 'activities', Content: InstanceActivities },
  ],
};
