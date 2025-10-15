import { Trash, SearchCode } from 'lucide-react';
import { useCallback } from 'react';
import { useVolumesContext } from './VolumesContext';
import { DockerVolumeResult } from '@/api/generated/api.types';
import { useNavigate } from 'react-router';
import { DropdownAction, DropdownActions } from '@/components/custom/dropdown-menu';

export const TableDropDown = ({ volume }: { volume: DockerVolumeResult }) => {
  const navigate = useNavigate();
  const { setDialogData } = useVolumesContext();

  const openDialog = useCallback(() => {
    setDialogData({ open: true, currentSelection: [volume] });
  }, [setDialogData, volume]);

  const items: DropdownAction[] = [
    {
      id: 'inspect',
      label: 'Inspect',
      icon: <SearchCode className="mr-2 h-3 w-3" />,
      onClick: () => navigate(volume.id),
    },
    {
      id: 'delete',
      label: 'Delete',
      icon: <Trash className="mr-2 h-3 w-3 text-danger" />,
      onClick: openDialog,
      disabled: volume.inUse ?? false,
      danger: true,
    },
  ];

  return <DropdownActions items={items} />;
};
