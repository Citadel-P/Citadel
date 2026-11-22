import { ServerCog } from 'lucide-react';
import type { TabbedResourceComponents } from '@/pages/types';
import { BackToTop } from '@/components/custom/back-to-top';
import { InstanceRestartButton } from './restart';
import { InstanceJobs } from './jobs';
import { InstanceSchedules } from './schedules';
import { InstanceActivities } from './activities';
import { InstanceSettingsForm } from './settings';

export const InstanceComponents: TabbedResourceComponents = {
  Icon: ServerCog,
  access: 'administrator',
  showTaskSheet: true,
  tabsVariant: 'bordered',
  SubHeader: BackToTop,
  header: {
    subtitle: 'Configuration and runtime diagnostics for this Core.',
    variant: 'plain',
    showSearch: false,
    showAdd: false,
  },
  Tabs: [
    { label: 'Config', slug: 'config', Content: InstanceSettingsForm, Header: { Extra: InstanceRestartButton } },
    { label: 'Jobs', slug: 'jobs', Content: InstanceJobs },
    { label: 'Schedules', slug: 'schedules', Content: InstanceSchedules },
    { label: 'Activities', slug: 'activities', Content: InstanceActivities },
  ],
};
