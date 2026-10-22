import { ProblemDetails } from '@/api/generated/api.types';
import { AlertMessage } from '@/components/custom/alert-message';
import { useRead } from '@/lib/hooks';
import { InspectionPanel } from '@/components/custom/inspection-panel';
import { SwarmServiceInfoView } from '../hooks/useServicesGroup';
import { ServiceProcesses } from './processes';

type InspectableService = Pick<SwarmServiceInfoView, 'id' | 'platformId'>;

export const ServiceInspect = ({ service }: { service: InspectableService }) => {
  const query = useRead('inspectSwarmService', { platformId: service.platformId, resourceId: service.id });
  const problem = (query.error as { error?: ProblemDetails } | undefined)?.error;
  if (problem) {
    return (
      <AlertMessage title={problem.title ?? 'Unable to inspect service'} type="error">
        {problem.detail ?? 'Docker could not inspect this service.'}
      </AlertMessage>
    );
  }

  return (
    <InspectionPanel
      data={query.data?.data}
      isLoading={query.isLoading}
      filename={`inspect-service-${service.id}.json`}
      loadingMessage="// Loading service inspection data..."
      noDataMessage="// No service inspection data available">
      <ServiceProcesses platformId={service.platformId} serviceId={service.id} />
    </InspectionPanel>
  );
};
