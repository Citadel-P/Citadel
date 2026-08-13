import { ServiceAccountView } from '@/api/generated/api.types';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { Archive, Pencil } from 'lucide-react';
import { useNavigate } from 'react-router';

export const {
  dropdown: ServiceAccountDropdownActions,
  group: ServiceAccountGroupActions,
  info: ServiceAccountInfoActions,
} =
  createActionsBuilder<ServiceAccountView>()
    .addAction({
      key: 'edit',
      type: 'command',
      icon: Pencil,
      useHandler: ({ resources }) => {
        const navigate = useNavigate();
        const selected = Array.isArray(resources) ? resources[0] : resources;
        const canExecute = !!selected && (!Array.isArray(resources) || resources.length === 1);
        return {
          canExecute,
          isPending: false,
          run: () => canExecute && selected && navigate(`/access/service-accounts/edit/${selected.id}`),
        };
      },
    })
    .addAction({
      key: 'archive',
      type: 'command',
      icon: Archive,
      mutateKey: 'archiveServiceAccounts',
      invalidate: 'listServiceAccounts',
      canExecute: (resources) => {
        const selected = Array.isArray(resources) ? resources : [resources];
        return selected.length > 0 && selected.every((account) => !account.archivedAtUtc);
      },
      separatorBefore: true,
      confirm: true,
      confirmationDescription: 'Archiving is irreversible. Every unrevoked credential will be permanently revoked.',
      destructive: true,
      resourceType: 'ServiceAccount',
      useVariables: (resources) => {
        const selected = Array.isArray(resources) ? resources : [resources];
        return { ids: selected.map((account) => account.id) };
      },
      useSuccessHandler: () => {
        const navigate = useNavigate();
        return () => navigate('/access/service-accounts');
      },
    })
    .build();
