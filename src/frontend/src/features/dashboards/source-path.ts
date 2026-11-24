import type { DashboardMetricSource, DashboardSourceView } from '@/api/generated/api.types';
export const sourcePath = (kind: DashboardMetricSource['kind'], row: DashboardSourceView) => {
  switch (kind) {
    case 'Container':
      return '/platforms/' + row.platformId + '/containers/' + row.id;
    case 'Platform':
      return '/platforms/edit/' + row.id;
    case 'SwarmService':
      return '/swarm-services/edit/' + row.id;
    case 'Stack':
      return '/stacks/edit/' + row.id;
    case 'Deployment':
      return '/deployments/edit/' + row.id;
  }
};
