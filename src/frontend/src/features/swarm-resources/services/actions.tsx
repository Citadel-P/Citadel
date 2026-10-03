import { ResourceControlState, StackImportKind, SwarmServiceOwnership } from '@/api/generated/api.types';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { useTaskSheet } from '@/lib/atoms';
import { useMutate } from '@/lib/hooks';
import { hasCapability } from '@/lib/resource-capabilities';
import { Eye, Layers3, PackagePlus, RefreshCw, Trash } from 'lucide-react';
import { useNavigate, useParams } from 'react-router';
import { toast } from 'sonner';
import { SwarmServiceListView } from './hooks/useServicesGroup';

export const isAdoptableService = (service: SwarmServiceListView) =>
  service.canAdopt === true &&
  !service.managedServiceId &&
  !service.isManagedDraft &&
  !service.isStale &&
  service.ownership === SwarmServiceOwnership.Unmanaged;

export const isImportableStackService = (service: SwarmServiceListView) =>
  !service.isStale &&
  service.ownership === SwarmServiceOwnership.DockerStackExternal &&
  Boolean(service.dockerStackNamespace);

const isManagedService = (service: SwarmServiceListView) => !!service.managedServiceId;
const isIdleManagedService = (service: SwarmServiceListView) =>
  isManagedService(service) && service.managedControlState !== ResourceControlState.Processing;
const asSelection = (resources: SwarmServiceListView | SwarmServiceListView[]) =>
  Array.isArray(resources) ? resources : [resources];

export const getServiceViewRoute = (service: SwarmServiceListView, platformId: string) =>
  service.isManagedDraft && service.managedServiceId
    ? `/swarm-services/edit/${service.managedServiceId}`
    : `/platforms/${platformId}/services/${service.id}`;

const serviceActions = createActionsBuilder<SwarmServiceListView>()
  .addAction({
    key: 'view',
    title: 'View',
    type: 'command',
    icon: Eye,
    requiredCapabilities: ['canRead'],
    useHandler: ({ resources }) => {
      const navigate = useNavigate();
      const { platformId } = useParams<{ platformId: string }>();
      const selection = asSelection(resources);
      const service = selection[0];
      const canExecute = !!platformId && selection.length === 1 && !!service;
      return {
        canExecute,
        isPending: false,
        run: () => {
          if (!canExecute || !platformId) return;
          navigate(getServiceViewRoute(service, platformId));
        },
      };
    },
  })
  .addAction({
    key: 'adopt',
    title: 'Adopt Service',
    type: 'command',
    icon: PackagePlus,
    requiredCapabilities: [],
    useHandler: ({ resources }) => {
      const navigate = useNavigate();
      const { platformId } = useParams<{ platformId: string }>();
      const selection = asSelection(resources);
      const service = selection[0];
      const canExecute = !!platformId && selection.length === 1 && !!service && isAdoptableService(service);
      const disabledReason = getAdoptDisabledReason(service, selection.length);
      return {
        canExecute,
        disabledReason,
        isPending: false,
        run: () => {
          if (!canExecute || !platformId) return;
          navigate(`/platforms/${platformId}/services/add?adoptFrom=${encodeURIComponent(service.id)}`);
        },
      };
    },
  })
  .addAction({
    key: 'importStack',
    title: 'Import Stack',
    type: 'command',
    icon: Layers3,
    requiredCapabilities: ['canInspect'],
    useHandler: ({ resources }) => {
      const navigate = useNavigate();
      const { platformId } = useParams<{ platformId: string }>();
      const selection = asSelection(resources);
      const service = selection[0];
      const canExecute = !!platformId && selection.length === 1 && !!service && isImportableStackService(service);
      return {
        canExecute,
        disabledReason: canExecute ? undefined : 'Select one unmanaged Docker Stack Service to import its Stack.',
        isPending: false,
        run: () => {
          if (!canExecute || !platformId || !service.dockerStackNamespace) return;
          navigate(
            `/stacks/add?importPlatform=${encodeURIComponent(platformId)}&importProject=${encodeURIComponent(service.dockerStackNamespace)}&importKind=${StackImportKind.SwarmStack}`,
          );
        },
      };
    },
  })
  .addAction({
    key: 'restart',
    title: 'Restart Service',
    type: 'command',
    icon: RefreshCw,
    confirm: true,
    resourceType: 'Service',
    requiredCapabilities: [],
    useHandler: ({ resources }) => {
      const { open } = useTaskSheet('SwarmService');
      const { platformId } = useParams<{ platformId: string }>();
      const { mutateAsync, isPending } = useMutate('restartSwarmService');
      const selection = asSelection(resources);
      const service = selection[0];
      const canExecute = canRestart(service, selection.length, !!platformId);
      return {
        canExecute,
        isPending,
        run: async () => {
          if (!canExecute || !service || !platformId) return;
          if (service.managedServiceId) {
            open({
              kind: 'swarmService',
              payload: {
                id: service.managedServiceId,
                name: service.name,
                action: 'force-update',
              },
            });
            return;
          }

          await mutateAsync({ platformId, resourceId: service.id });
          toast.success(`Restart requested for ${service.name}`);
        },
      };
    },
  })
  .addAction({
    key: 'delete',
    title: 'Delete',
    type: 'command',
    icon: Trash,
    destructive: true,
    confirm: true,
    resourceType: 'Service',
    requiredCapabilities: [],
    useHandler: ({ resources }) => {
      const { platformId } = useParams<{ platformId: string }>();
      const managedMutation = useMutate('deleteSwarmServices');
      const nativeMutation = useMutate('deleteSwarmInventoryServices');
      const selection = asSelection(resources);
      const managed = selection.every(isManagedService);
      const native = selection.every((service) => !isManagedService(service));
      const permitted = selection.every((service) => hasCapability(service, 'canExecute'));
      const available = selection.every((service) =>
        isManagedService(service) ? isIdleManagedService(service) : !service.isStale,
      );
      const canExecute =
        !!platformId && selection.length > 0 && (managed || native) && permitted && available;
      return {
        canExecute,
        isPending: managedMutation.isPending || nativeMutation.isPending,
        run: async () => {
          if (!canExecute || !platformId) return;
          if (managed) {
            await managedMutation.mutateAsync({
              data: selection.map((service) => service.managedServiceId!),
            });
          } else {
            await nativeMutation.mutateAsync({
              platformId,
              data: { ids: selection.map((service) => service.id) },
            });
          }
          toast.success(`${selection.length === 1 ? 'Service' : 'Services'} deleted`);
        },
      };
    },
  })
  .build();

const getAdoptDisabledReason = (
  service: SwarmServiceListView | undefined,
  selectionCount: number,
): string | undefined => {
  if (selectionCount !== 1) return 'Select one Service to adopt.';
  if (!service) return 'Select a Service to adopt.';
  if (service.managedServiceId) return 'This Service is already managed by Citadel.';
  if (service.isStale) return 'Refresh the stale Service inventory before adoption.';
  if (service.ownership !== SwarmServiceOwnership.Unmanaged)
    return 'Only unmanaged standalone Services can be adopted.';
  if (!service.canAdopt || !hasCapability(service, 'canInspect'))
    return 'You do not have permission to adopt this Service.';
  return undefined;
};

const canRestart = (
  service: SwarmServiceListView | undefined,
  selectionCount: number,
  hasPlatformId: boolean,
) => {
  if (!hasPlatformId || selectionCount !== 1 || !service || service.isStale) return false;
  if (service.managedServiceId)
    return !service.isManagedDraft && isIdleManagedService(service) && hasCapability(service, 'canApply');
  return hasCapability(service, 'canExecute');
};

export const ServiceDropdownActions = serviceActions.dropdown;
export const ServiceGroupActions = serviceActions.group;
export const ServiceInfoActions = serviceActions.info;
