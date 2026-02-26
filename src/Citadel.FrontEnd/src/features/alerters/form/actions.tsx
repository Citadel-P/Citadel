import { Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { RegistryView } from '@/api/generated/api.types';

export const { info: AlertRuleActions } = createActionsBuilder<RegistryView>()
  .addAction({
    key: 'delete',
    type: 'command',
    icon: Trash,
    mutateKey: 'deleteRegistries',
    confirm: true,
    destructive: true,
    resourceType: 'Registry',
    canExecute: () => true,
    useVariables: (resource) => {
      return {
        ids: [(resource as RegistryView).id],
      };
    },
    useSuccessHandler: () => {
      const navigate = useNavigate();
      return () => {
        navigate(`/alerters`);
      };
    },
  })
  .build();
