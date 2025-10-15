import { Pencil, Trash } from 'lucide-react';
import { RegistryView } from '@/api/generated/api.types';
import { useRegistriesContext } from './RegistriesContext';
import { createTableDropdown } from '@/components/custom/dropdown-menu';

export const TableDropdown = ({ registry }: { registry: RegistryView }) => {
  const context = useRegistriesContext();

  return createTableDropdown({
    resource: registry,
    context,
    actions: ({ navigate, openDialog }) => [
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
    ],
  });
};
