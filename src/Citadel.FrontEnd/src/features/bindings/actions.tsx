import { ResourceBindingView } from '@/api/generated/api.types';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { useSelectedResources } from '@/lib/atoms';
import { useMutate } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { Pencil, Trash2 } from 'lucide-react';
import { toast } from 'sonner';

export type BindingPageAction =
  | { type: 'add-variable' }
  | { type: 'add-secret-key' }
  | { type: 'create-secret' }
  | { type: 'edit-entry'; entry: ResourceBindingView };

const bindingActions = new EventTarget();

export const dispatchBindingPageAction = (action: BindingPageAction) => {
  bindingActions.dispatchEvent(new CustomEvent<BindingPageAction>('bindings-action', { detail: action }));
};

export const subscribeBindingPageActions = (listener: (action: BindingPageAction) => void) => {
  const wrapped = (event: Event) => listener((event as CustomEvent<BindingPageAction>).detail);
  bindingActions.addEventListener('bindings-action', wrapped);
  return () => bindingActions.removeEventListener('bindings-action', wrapped);
};

export const invalidateBindingQueries = async (queryClient: ReturnType<typeof useQueryClient>) => {
  await queryClient.invalidateQueries({ queryKey: ['getGlobalResourceBindings'] });
  await queryClient.invalidateQueries({ queryKey: ['getResourceBindings'] });
  await queryClient.invalidateQueries({ queryKey: ['listSecretDefinitions'] });
};

const { dropdown, group } = createActionsBuilder<ResourceBindingView>()
  .addAction({
    key: 'edit',
    type: 'command',
    icon: Pencil,
    requiredCapabilities: ['canWrite'],
    useHandler: ({ resources }) => {
      const selected = Array.isArray(resources) ? resources[0] : resources;
      const multiSelect = Array.isArray(resources) && resources.length > 1;

      return {
        canExecute: !!selected && !multiSelect,
        run: () => {
          if (!selected || multiSelect) return;
          dispatchBindingPageAction({ type: 'edit-entry', entry: selected });
        },
      };
    },
  })
  .addAction({
    key: 'delete',
    type: 'command',
    icon: Trash2,
    confirm: true,
    destructive: true,
    resourceType: 'Binding',
    requiredCapabilities: ['canWrite'],
    useHandler: ({ resources }) => {
      const selected = Array.isArray(resources) ? resources : [resources];
      const queryClient = useQueryClient();
      const remove = useMutate('deleteGlobalResourceBinding');
      const [, setSelectedResources] = useSelectedResources<ResourceBindingView>('Binding');

      return {
        canExecute: selected.length > 0,
        isPending: remove.isPending,
        run: async () => {
          if (selected.length === 0) return;

          try {
            await Promise.all(selected.map((entry) => remove.mutateAsync({ id: entry.id } as any)));
            await invalidateBindingQueries(queryClient);
            setSelectedResources([]);
            toast.success(`${selected.length} ${selected.length === 1 ? 'entry' : 'entries'} deleted`);
          } catch (error) {
            toast.error(remove.validationErrors ?? 'Failed to delete selected entries');
            throw error;
          }
        },
      };
    },
  })
  .build();

export const BindingDropdownActions = dropdown;
export const BindingGroupActions = group;
