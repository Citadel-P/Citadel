import { DropdownMenu, DropdownMenuContent, DropdownMenuTrigger } from '@/components/ui/dropdown-menu';
import { Button } from '@/components/ui/button';
import { MoreHorizontal, Trash } from 'lucide-react';
import { useContextSelector } from 'use-context-selector';
import { useCallback } from 'react';
import { ActionMenuItem } from '@/components/ui/ActionMenuItem';
import { NetworksContext } from './NetworksProvider';
import { NetworkView } from '@/api/_generated';

const DropdownTableMenu = ({ network }: { network: NetworkView }) => {
  const setDialogData = useContextSelector(NetworksContext, (v) => v?.setDialogData)!;

  const openDialog = useCallback(() => {
    setDialogData({ open: true, currentSelection: [network] });
  }, [setDialogData, network]);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" className="h-8 w-8 p-0">
          <span className="sr-only">Open menu</span>
          <MoreHorizontal className="h-2 w-2" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-38 drop-shadow-md shadow-custom bg-background pt-2 pb-2">
        {/* Delete Action */}
        <ActionMenuItem
          onClick={openDialog}
          disabled={network.inUse ?? false}
          icon={<Trash className="mr-2 h-3 w-3 text-danger" />}
          label="Delete"
          className="text-danger"
        />
      </DropdownMenuContent>
    </DropdownMenu>
  );
};

export default DropdownTableMenu;
