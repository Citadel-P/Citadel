import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
  DropdownMenuSeparator,
} from '@/components/ui/dropdown-menu';
import { Button } from '@/components/ui/button';
import { MoreHorizontal, Pencil, Trash } from 'lucide-react';
import { RegistryView } from '@/api/generated/api.types';
import { useNavigate } from 'react-router';
import { useRegistriesContext } from './RegistriesContext';
import { useCallback } from 'react';
import { ActionMenuItem } from '@/components/ui/ActionMenuItem';

const DropdownTableMenu = ({ registry }: { registry: RegistryView }) => {
  const navigate = useNavigate();
  const { setDialogData } = useRegistriesContext();

  const openDialog = useCallback(() => {
    setDialogData({ open: true, currentSelection: [registry] });
  }, [setDialogData, registry]);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" className="h-8 w-8 p-0">
          <span className="sr-only">Open menu</span>
          <MoreHorizontal className="h-2 w-2" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-38 drop-shadow-md shadow-custom bg-background pt-2 pb-2">
        {/* Edit Action */}
        <ActionMenuItem
          onClick={() => navigate(`../registries/edit/${registry.id}`)}
          icon={<Pencil className="mr-2 h-3 w-3" />}
          label="Edit"
        />
        <DropdownMenuSeparator />
        {/* Delete Action */}
        <ActionMenuItem
          onClick={openDialog}
          icon={<Trash className="mr-2 h-3 w-3 text-danger" />}
          label="Delete"
          className="text-danger hover:text-danger!"
        />
      </DropdownMenuContent>
    </DropdownMenu>
  );
};

export default DropdownTableMenu;
