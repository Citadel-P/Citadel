import { DropdownMenu, DropdownMenuContent, DropdownMenuTrigger } from '@/components/ui/dropdown-menu';
import { Button } from '@/components/ui/button';
import { MoreHorizontal, Trash, SearchCode } from 'lucide-react';
import { useContextSelector } from 'use-context-selector';
import { useCallback } from 'react';
import { ActionMenuItem } from '@/components/ui/ActionMenuItem';
import { VolumesContext } from './VolumesProvider';
import { DockerVolumeResult } from '@/api/_generated';

const DropdownTableMenu = ({ volume }: { volume: DockerVolumeResult }) => {
  const setDialogData = useContextSelector(VolumesContext, (v) => v?.setDialogData)!;
  const setSheetOpen = useContextSelector(VolumesContext, (v) => v?.setSheetOpen)!;
  const setCurrentVolume = useContextSelector(VolumesContext, (v) => v?.setCurrentVolume)!;

  const openDialog = useCallback(() => {
    setDialogData({ open: true, currentSelection: [volume] });
  }, [setDialogData, volume]);

  const openSheet = useCallback(() => {
    setSheetOpen(true);
    setCurrentVolume(volume);
  }, [setSheetOpen, setCurrentVolume, volume]);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" className="h-8 w-8 p-0">
          <span className="sr-only">Open menu</span>
          <MoreHorizontal className="h-2 w-2" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-38 drop-shadow-md shadow-custom bg-background pt-2 pb-2">
        <ActionMenuItem onClick={openSheet} icon={<SearchCode className="mr-2 h-3 w-3" />} label="Inspect" />
        <ActionMenuItem
          onClick={openDialog}
          disabled={volume.inUse ?? false}
          icon={<Trash className="mr-2 h-3 w-3 text-danger" />}
          label="Delete"
          className="text-danger"
        />
      </DropdownMenuContent>
    </DropdownMenu>
  );
};

export default DropdownTableMenu;
