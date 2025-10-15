import { useVolumesContext } from './VolumesContext';
import { VolumeActionBarButtons } from './action-bar-buttons';
import { ActionBar } from '@/components/custom/action-bar';

export const VolumesActionBar = () => {
  const { setDialogData, selectedRows, volumes } = useVolumesContext();

  if (!selectedRows?.length) return null;

  return (
      <ActionBar
        selectedRows={selectedRows}
        allItems={volumes}
        resource="Volume"
        actionButtons={
          <VolumeActionBarButtons
            selectedRows={selectedRows}
            setDialogData={setDialogData}
          />
        }
      />
    );
};
