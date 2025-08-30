import { DropdownMenu, DropdownMenuContent, DropdownMenuTrigger } from '@/components/ui/dropdown-menu';
import { Button } from '@/components/ui/button';
import { MoreHorizontal, Trash, SearchCode } from 'lucide-react';
import { startTransition, useCallback } from 'react';
import { ActionMenuItem } from '@/components/ui/ActionMenuItem';
import { useNetworksContext } from './NetworksContext';
import { DockerNetworkResult } from '@/api/_generated';
import { useNavigate } from 'react-router';
import { formatId } from '@/lib/utils';

const DropdownTableMenu = ({ network }: { network: DockerNetworkResult }) => {
  const navigate = useNavigate();
  const { setDialogData } = useNetworksContext();
  const openDialog = useCallback(() => {
    startTransition(() => {
      setDialogData({ open: true, currentSelection: [network] });
    });
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
        <ActionMenuItem
          onClick={() => navigate(`${formatId(network.id)}`)}
          icon={<SearchCode className="mr-2 h-3 w-3" />}
          label="Inspect"
        />
        <ActionMenuItem
          onClick={openDialog}
          disabled={network.inUse ?? false}
          icon={<Trash className="mr-2 h-3 w-3 text-danger" />}
          label="Delete"
          className="text-danger hover:text-danger!"
        />
      </DropdownMenuContent>
    </DropdownMenu>
  );
};

export default DropdownTableMenu;
