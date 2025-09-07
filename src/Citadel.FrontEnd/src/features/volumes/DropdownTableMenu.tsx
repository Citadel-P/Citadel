import { DropdownMenu, DropdownMenuContent, DropdownMenuTrigger } from '@/components/ui/dropdown-menu';
import { Button } from '@/components/ui/button';
import { MoreHorizontal, Trash, SearchCode } from 'lucide-react';
import { useCallback } from 'react';
import { ActionMenuItem } from '@/components/ui/ActionMenuItem';
import { useVolumesContext } from './VolumesContext';
import { DockerVolumeResult } from '@/api/_generated';
import { useNavigate } from 'react-router';

const DropdownTableMenu = ({ volume }: { volume: DockerVolumeResult }) => {
  const navigate = useNavigate();
  const { setDialogData } = useVolumesContext();
  const openDialog = useCallback(() => {
    setDialogData({ open: true, currentSelection: [volume] });
  }, [setDialogData, volume]);

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
          onClick={() => navigate(volume.id)}
          icon={<SearchCode className="mr-2 h-3 w-3" />}
          label="Inspect"
        />
        <ActionMenuItem
          onClick={openDialog}
          disabled={volume.inUse ?? false}
          icon={<Trash className="mr-2 h-3 w-3 text-danger" />}
          label="Delete"
          className="text-danger hover:text-danger!"
        />
      </DropdownMenuContent>
    </DropdownMenu>
  );
};

export default DropdownTableMenu;
