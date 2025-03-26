import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  DropdownMenuSeparator,
} from '@/components/ui/dropdown-menu';
import { Button } from '@/components/ui/button';
import { MoreHorizontal, Pencil, Trash } from 'lucide-react';
import { RegistryView } from '@/api/_generated';
import { useNavigate } from 'react-router';
import { useContextSelector } from 'use-context-selector';
import { RegistriesContext } from './RegistriesProvider';

const DropdownTableMenu = ({ registry }: { registry: RegistryView }) => {
  const navigate = useNavigate();
  const setDialogData = useContextSelector(RegistriesContext, (v) => v?.setDialogData)!;

  function openDialog() {
    setDialogData({ open: true, currentSelection: [registry] });
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" className="h-8 w-8 p-0">
          <span className="sr-only">Open menu</span>
          <MoreHorizontal className="h-2 w-2" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-38 drop-shadow-md shadow-custom bg-background pt-2 pb-2">
        <DropdownMenuItem
          className="grow cursor-pointer rounded-sm px-3 py-2 text-xs font-semibold text-muted-foreground hover:bg-card"
          onClick={() => navigate(`../registries/edit/${registry.id}`)}>
          <Pencil className="mr-2" />
          <span>Edit</span>
        </DropdownMenuItem>

        <DropdownMenuSeparator />
        <DropdownMenuItem
          onClick={openDialog}
          className="grow cursor-pointer rounded-sm px-3 py-2 text-xs font-semibold ">
          <Trash className="mr-2 h-3 w-3 text-danger" />
          <span className="text-danger">Delete</span>
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
};

export default DropdownTableMenu;
