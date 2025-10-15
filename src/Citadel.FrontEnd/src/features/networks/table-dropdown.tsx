import { Trash, SearchCode } from 'lucide-react';
import { startTransition, useCallback } from 'react';
import { useNetworksContext } from './NetworksContext';
import { DockerNetworkResult } from '@/api/generated/api.types';
import { useNavigate } from 'react-router';
import { formatId } from '@/lib/utils';
import { DropdownAction, DropdownActions } from '@/components/custom/dropdown-menu';

export const TableDropDown = ({ network }: { network: DockerNetworkResult }) => {
  const navigate = useNavigate();
  const { setDialogData } = useNetworksContext();
  const openDialog = useCallback(() => {
    startTransition(() => {
      setDialogData({ open: true, currentSelection: [network] });
    });
  }, [setDialogData, network]);

  const items: DropdownAction[] = [
    {
      id: 'inspect',
      label: 'Inspect',
      icon: <SearchCode className="mr-2 h-3 w-3" />,
      onClick: () => navigate(`${formatId(network.id)}`),
    },
    {
      id: 'delete',
      label: 'Delete',
      icon: <Trash className="mr-2 h-3 w-3 text-danger" />,
      onClick: openDialog,
      disabled: network.inUse ?? false,
      danger: true,
    },
  ];

  return <DropdownActions items={items} />;
};
