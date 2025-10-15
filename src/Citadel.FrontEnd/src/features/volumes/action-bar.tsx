import { useVolumesContext } from './VolumesContext';
import { ActionBarButtons } from './action-bar-buttons';
import { GenericActionBar } from '@/components/custom/action-bar';

export const ActionBar = () => {
  const { setDialogData, selectedRows, volumes } = useVolumesContext();

  if (!selectedRows?.length) return null;

  return (
    <GenericActionBar
      selectedRows={selectedRows}
      allItems={volumes}
      resource="Volume"
      actionButtons={<ActionBarButtons selectedRows={selectedRows} setDialogData={setDialogData} />}
    />
  );
};
