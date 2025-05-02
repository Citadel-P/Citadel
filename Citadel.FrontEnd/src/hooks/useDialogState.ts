import { useState } from 'react';

export function useDialogState<T>() {
  const [dialogData, setDialogData] = useState<IDeleteDialogData<T>>({ open: false });

  return { dialogData, setDialogData };
}

export interface IDeleteDialogData<T> {
  open: boolean;
  currentSelection?: T[];
}
