import { Server } from 'lucide-react';
import type { RegularResourceComponents } from '@/pages/types';
import { InstanceSettingsForm } from './settings';

export const InstanceComponents: RegularResourceComponents = {
  Icon: Server,
  access: 'administrator',
  showTaskSheet: false,
  header: {
    subtitle: 'Configure security, monitoring and Core execution defaults.',
    variant: 'plain',
    showSearch: false,
    showAdd: false,
  },
  Content: InstanceSettingsForm,
};
