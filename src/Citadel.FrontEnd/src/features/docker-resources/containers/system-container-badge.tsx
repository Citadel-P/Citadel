import { ContainerSystemRole } from '@/api/generated/api.types';
import { SystemBadge } from '@/components/custom/system-badge';

export function SystemContainerBadge({ role, stack = false }: { role?: ContainerSystemRole | null; stack?: boolean }) {
  const description = stack
    ? 'Citadel system stack'
    : role === ContainerSystemRole.Core
      ? 'Citadel Core'
      : role === ContainerSystemRole.Database
        ? 'Citadel database'
        : role === ContainerSystemRole.Agent
          ? 'Citadel Agent'
          : role === ContainerSystemRole.EdgeAgent
            ? 'Citadel Edge Agent'
            : 'Citadel system container';

  return <SystemBadge description={description} ariaLabel={stack ? 'System stack' : description} />;
}
