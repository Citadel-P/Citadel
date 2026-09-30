import { Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { AuthorizedRegistryView } from '@/api/generated/api.types';

export const { info: AlertRuleActions } = createActionsBuilder<AuthorizedRegistryView>()
  .addAction({
    key: 'delete',
    type: 'command',
    icon: Trash,
    mutateKey: 'deleteAlertRules',
    confirm: true,
    destructive: true,
    resourceType: 'AlertRule',
    canExecute: () => true,
    useVariables: (resource) => {
      return {
        ids: [(resource as AuthorizedRegistryView).id],
      };
    },
    useSuccessHandler: () => {
      const navigate = useNavigate();
      return () => {
        navigate(`/alert-rules`);
      };
    },
  })
  .build();
