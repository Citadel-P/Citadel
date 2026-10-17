import { Activity } from 'lucide-react';
import { ContainerHealthStatus, type ContainerView } from '@/api/generated/api.types';
import { HealthCard } from '@/components/custom/health-card';
import { toContainerDetailsView, type ContainerDetailsView } from '../hooks/useContainerInfoGroup';
import {
  ContainerHealthSummary,
  currentHealth,
  HealthLabel,
  healthLabels,
  healthTone,
  useHealthClock,
} from '../health';
import { ContainerHealth } from './container-health';

export function ContainerHealthCard({
  resource,
  initiallyOpen = false,
  iconOnly = false,
}: {
  resource: ContainerDetailsView;
  initiallyOpen?: boolean;
  iconOnly?: boolean;
}) {
  const status = currentHealth(resource, useHealthClock());
  const tone = healthTone(status);
  return (
    <HealthCard
      title="Container health"
      resourceName={resource.name}
      triggerLabel={`Health: ${healthLabels[status]}`}
      triggerClassName={tone}
      trigger={
        iconOnly ? (
          <Activity className={`size-4 ${tone}`} aria-hidden="true" />
        ) : status === ContainerHealthStatus.NotRunning ? (
          <>
            <Activity className={`size-4 ${tone}`} aria-hidden="true" />
            <span>—</span>
          </>
        ) : (
          <HealthLabel status={status} />
        )
      }
      status={<HealthLabel status={status} />}
      summary={<ContainerHealthSummary container={resource} />}
      initiallyOpen={initiallyOpen}
      renderDetails={() => <ContainerHealth resource={resource} />}
    />
  );
}

export function ContainerTableHealth({ container }: { container: ContainerView }) {
  return <ContainerHealthCard key={container.id} resource={toContainerDetailsView(container)} iconOnly />;
}
