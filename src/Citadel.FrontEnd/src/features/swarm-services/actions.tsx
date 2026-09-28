import { Copy, Eye, Rocket, RefreshCw, Zap, Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { ManagedSwarmServiceView, ResourceControlState } from '@/api/generated/api.types';
import { createActionsBuilder, ActionConfig } from '@/components/custom/actions-builder';
import { useTaskSheet } from '@/lib/atoms';
import { useMutate } from '@/lib/hooks';
import { toast } from 'sonner';

const ids = (value: ManagedSwarmServiceView | ManagedSwarmServiceView[]) =>
  Array.isArray(value) ? value.map((item) => item.id) : [value.id];
const idle = (value: ManagedSwarmServiceView) => value.controlState !== ResourceControlState.Processing;

export const applySwarmServiceAction: ActionConfig<ManagedSwarmServiceView, any> = {
  key: 'apply',
  type: 'command',
  icon: Rocket,
  title: 'Apply',
  confirm: true,
  resourceType: 'SwarmService',
  requiredCapabilities: ['canRead', 'canApply'],
  useHandler: ({ resources }) => {
    const { open } = useTaskSheet('SwarmService');
    const service = Array.isArray(resources) ? resources[0] : resources;
    const canExecute = !!service && !Array.isArray(resources) && idle(service);
    return {
      canExecute,
      isPending: false,
      run: () => {
        if (canExecute)
          open({ kind: 'swarmService', payload: { id: service.id, name: service.name, action: 'apply' } });
      },
    };
  },
};

export const forceUpdateSwarmServiceAction: ActionConfig<ManagedSwarmServiceView, any> = {
  key: 'forceUpdate',
  type: 'command',
  icon: Zap,
  title: 'Restart Tasks',
  confirm: true,
  resourceType: 'SwarmService',
  requiredCapabilities: ['canRead', 'canApply'],
  useHandler: ({ resources }) => {
    const { open } = useTaskSheet('SwarmService');
    const service = Array.isArray(resources) ? resources[0] : resources;
    const canExecute = !!service && !Array.isArray(resources) && !!service.dockerServiceId && idle(service);
    return {
      canExecute,
      isPending: false,
      run: () => {
        if (canExecute)
          open({ kind: 'swarmService', payload: { id: service.id, name: service.name, action: 'force-update' } });
      },
    };
  },
};

export const duplicateSwarmServiceAction: ActionConfig<ManagedSwarmServiceView, any> = {
  key: 'duplicate',
  type: 'command',
  icon: Copy,
  requiredCapabilities: ['canRead', 'canWrite'],
  useHandler: ({ resources }) => {
    const navigate = useNavigate();
    const service = Array.isArray(resources) ? resources[0] : resources;
    const multiple = Array.isArray(resources) && resources.length > 1;
    return {
      canExecute: !!service && !multiple,
      isPending: false,
      run: () => {
        if (!service || multiple) return;
        navigate(`/platforms/${service.platformId}/services/add?duplicateFrom=${service.id}`);
      },
    };
  },
};

export const {
  dropdown: SwarmServiceDropdownActions,
  group: SwarmServiceGroupActions,
  info: SwarmServiceInfoActions,
} = createActionsBuilder<ManagedSwarmServiceView>()
  .addAction(applySwarmServiceAction)
  .addAction(forceUpdateSwarmServiceAction)
  .addAction(duplicateSwarmServiceAction)
  .addAction({
    key: 'checkUpdates',
    type: 'command',
    icon: RefreshCw,
    title: 'Check for Updates',
    mutateKey: 'checkSwarmServiceUpdates',
    requiredCapabilities: ['canWrite'],
    useHandler: ({ resources }) => {
      const service = Array.isArray(resources) ? resources[0] : resources;
      const multiple = Array.isArray(resources) && resources.length > 1;
      const { mutateAsync, isPending } = useMutate('checkSwarmServiceUpdates');
      const disabledReason = getUpdateCheckDisabledReason(service, multiple);
      return {
        canExecute: disabledReason === undefined,
        disabledReason,
        isPending,
        run: async () => {
          if (!service || disabledReason) return;
          const response = await mutateAsync({ id: service.id });
          if (response.data.autoUpdateState.status === 'UpdateAvailable')
            toast.info('Image update available', { description: 'A newer image digest is available.' });
          else toast.success('Image is up to date', { description: 'No newer image digest was found.' });
        },
      };
    },
  })
  .addAction({
    key: 'details',
    type: 'command',
    icon: Eye,
    separatorBefore: true,
    useHandler: ({ resources }) => {
      const navigate = useNavigate();
      const service = Array.isArray(resources) ? resources[0] : resources;
      return {
        canExecute: !!service && !Array.isArray(resources),
        isPending: false,
        run: () => service && navigate(`/swarm-services/edit/${service.id}`),
      };
    },
  })
  .addAction({
    key: 'delete',
    type: 'command',
    icon: Trash,
    mutateKey: 'deleteSwarmServices',
    destructive: true,
    confirm: true,
    separatorBefore: true,
    resourceType: 'SwarmService',
    requiredCapabilities: ['canExecute'],
    canExecute: (service) => (Array.isArray(service) ? service.every(idle) : idle(service)),
    useVariables: ids,
    useSuccessHandler: () => {
      const navigate = useNavigate();
      return () => navigate('/swarm-services');
    },
  })
  .build();

const getUpdateCheckDisabledReason = (
  service: ManagedSwarmServiceView | undefined,
  multiple: boolean,
): string | undefined => {
  if (multiple) return 'Select one Service to check its image.';
  if (!service) return 'Select a Service to check its image.';
  if (!idle(service)) return 'Wait for the current Service operation to finish.';
  if (service.spec.image.$type !== 'External') return 'Image checks require an external tagged image.';
  if (service.spec.image.imageTag.includes('@')) return 'Digest-pinned images do not support image checks.';
  if (!service.appliedImageDigest) return 'Deploy this Service once so Citadel has an applied digest to compare.';
  return undefined;
};
