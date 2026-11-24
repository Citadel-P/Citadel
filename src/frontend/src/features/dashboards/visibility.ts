import { DashboardVisibility } from '@/api/generated/api.types';

export const audiences = {
  [DashboardVisibility.InstanceShared]: 'Everyone in this instance',
  [DashboardVisibility.Restricted]: 'Restricted',
  [DashboardVisibility.Private]: 'Private',
};
