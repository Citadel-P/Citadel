import { Pencil, Trash } from 'lucide-react';
import { RegistryView } from '@/api/generated/api.types';
import { useNavigate } from 'react-router';
import { useRegistriesContext } from './RegistriesContext';
import { useCallback } from 'react';
import { DropdownAction, DropdownActions } from '@/components/custom/dropdown-menu';

export const TableDropdown = ({ registry }: { registry: RegistryView }) => {
  const navigate = useNavigate();
  const { setDialogData } = useRegistriesContext();

  const openDialog = useCallback(() => {
    setDialogData({ open: true, currentSelection: [registry] });
  }, [setDialogData, registry]);

  const items: DropdownAction[] = [
    {
      id: 'edit',
      label: 'Edit',
      icon: <Pencil className="mr-2 h-3 w-3" />,
      onClick: () => navigate(`../registries/edit/${registry.id}`),
    },
    {
      id: 'delete',
      label: 'Delete',
      icon: <Trash className="mr-2 h-3 w-3 text-danger" />,
      onClick: openDialog,
      danger: true,
      separatorBefore: true,
    },
  ];

  return <DropdownActions items={items} />;
};
