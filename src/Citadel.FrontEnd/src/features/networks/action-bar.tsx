import { useNetworksContext } from './NetworksContext';
import { NetworkActionBarButtons } from './action-bar-buttons';
import { ActionBar } from '@/components/custom/action-bar';

export const NetworksActionBar = () => {
  const { setDialogData, selectedRows, networks } = useNetworksContext();

  const canInspect = selectedRows?.length === 1;

  return (
    <ActionBar
      selectedRows={selectedRows}
      allItems={networks}
      resource="Network"
      actionButtons={
        <NetworkActionBarButtons
          selectedRows={selectedRows}
          setDialogData={setDialogData}
          showInspectButton={canInspect}
        />
      }
    />
  );
};
