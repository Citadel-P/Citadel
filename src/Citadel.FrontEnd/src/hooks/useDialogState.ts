import { useState } from 'react';

/**
 * A React hook to manage the state of a dialog.
 *
 * @template T The type of the data associated with the dialog, typically the items to be affected.
 * @returns An object containing the dialog state and a function to update it.
 */
export function useDialogState<T>() {
  const [dialogData, setDialogData] = useState<IDialogData<T>>({ open: false });

  return { dialogData, setDialogData };
}

/**
 * Interface for the state of a dialog.
 * @template T The type of the items in the current selection.
 */
export interface IDialogData<T> {
  /** Whether the dialog is open or not. */
  open: boolean;
  /** The currently selected items, typically for performing an action like deletion. */
  currentSelection?: T[];
}
