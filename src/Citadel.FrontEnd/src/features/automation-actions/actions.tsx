import { AuthorizedAction, ResourceControlState } from '@/api/generated/api.types';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { useSelectedResources, useTaskSheet } from '@/lib/atoms';
import { useMutate } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { Pencil, Play, Trash2 } from 'lucide-react';
import { useLocation, useNavigate } from 'react-router';
import { toast } from 'sonner';

export const invalidateAutomationActionQueries = async (
  queryClient: ReturnType<typeof useQueryClient>,
  id?: string,
) => {
  await queryClient.invalidateQueries({ queryKey: ['listAutomationActions'] });
  if (id) {
    await queryClient.invalidateQueries({ queryKey: ['getAutomationAction', { id }] });
    await queryClient.invalidateQueries({ queryKey: ['listAutomationActionRuns', { id }] });
  } else {
    await queryClient.invalidateQueries({ queryKey: ['getAutomationAction'] });
    await queryClient.invalidateQueries({ queryKey: ['listAutomationActionRuns'] });
  }
};

const singleSelection = (resources: AuthorizedAction | AuthorizedAction[]) => {
  const selected = Array.isArray(resources) ? resources[0] : resources;
  const multiSelect = Array.isArray(resources) && resources.length > 1;
  return { selected, multiSelect };
};

const useAutomationRunTaskSheet = () => {
  const automationSheet = useTaskSheet('Automation');
  const automationActionSheet = useTaskSheet('AutomationAction');
  const location = useLocation();
  const isFormRoute = /\/automation\/(?:actions\/)?(?:add|edit)(?:\/|$)/.test(location.pathname);

  return isFormRoute ? automationActionSheet : automationSheet;
};

const { dropdown, group, info } = createActionsBuilder<AuthorizedAction>()
  .addAction({
    key: 'edit',
    type: 'command',
    icon: Pencil,
    requiredCapabilities: ['canWrite'],
    useHandler: ({ resources }) => {
      const { selected, multiSelect } = singleSelection(resources);
      const navigate = useNavigate();

      return {
        canExecute: !!selected && !multiSelect,
        run: () => {
          if (!selected || multiSelect) return;
          navigate(`/automation/edit/${selected.id}`);
        },
      };
    },
  })
  .addAction({
    key: 'run',
    type: 'command',
    icon: Play,
    requiredCapabilities: ['canExecute'],
    useHandler: ({ resources }) => {
      const { selected, multiSelect } = singleSelection(resources);
      const { open: openSheet } = useAutomationRunTaskSheet();
      const blocked = selected?.controlState === ResourceControlState.Processing;

      return {
        canExecute: !!selected && !multiSelect && selected.enabled && !blocked,
        run: () => {
          if (!selected || multiSelect) return;
          openSheet({
            kind: 'automationActionRun',
            payload: {
              id: selected.id,
              name: selected.name,
              mode: 'run',
              argsJson: null,
              timeoutSeconds: null,
            },
          });
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
    resourceType: 'AutomationAction',
    requiredCapabilities: ['canWrite'],
    useHandler: ({ resources }) => {
      const selected = Array.isArray(resources) ? resources : [resources];
      const queryClient = useQueryClient();
      const remove = useMutate('deleteAutomationAction');
      const [, setSelectedResources] = useSelectedResources<AuthorizedAction>('AutomationAction');
      const location = useLocation();
      const navigate = useNavigate();
      const isCurrentResource = selected.some(
        (action) => location.pathname.replace(/\/$/, '') === `/automation/edit/${action.id}`,
      );

      return {
        canExecute: selected.length > 0,
        isPending: remove.isPending,
        run: async () => {
          if (selected.length === 0) return;

          try {
            await Promise.all(selected.map((action) => remove.mutateAsync({ id: action.id } as any)));
            if (isCurrentResource) navigate('/automation', { replace: true });
            await queryClient.invalidateQueries({ queryKey: ['listAutomationActions'] });
            setSelectedResources([]);
            toast.success(`${selected.length} ${selected.length === 1 ? 'action' : 'actions'} deleted`);
          } catch (error) {
            toast.error(remove.validationErrors ?? 'Failed to delete selected actions');
            throw error;
          }
        },
      };
    },
  })
  .build();

export const AutomationActionDropdownActions = dropdown;
export const AutomationActionGroupActions = group;
export const AutomationActionInfoActions = info;
