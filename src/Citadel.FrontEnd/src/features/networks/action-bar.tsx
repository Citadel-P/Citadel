import { useNetworksContext } from './NetworksContext';
import { ActionBarButtons } from './action-bar-buttons';
import { GenericActionBar } from '@/components/custom/action-bar';

export const ActionBar = () => {
  const { setDialogData, selectedRows, networks } = useNetworksContext();

  const canInspect = selectedRows?.length === 1;

  return (
    <GenericActionBar
      selectedRows={selectedRows}
      allItems={networks}
      resource="Network"
      actionButtons={
        <ActionBarButtons selectedRows={selectedRows} setDialogData={setDialogData} showInspectButton={canInspect} />
      }
    />
  );
};
