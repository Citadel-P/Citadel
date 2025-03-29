import { ContainerInfoView } from '@/api/_generated';
import { useState } from 'react';

export function useDialogState() {
  const [dialogData, setDialogData] = useState<IDeleteDialogData>({ open: false });

  return { dialogData, setDialogData };
}

export interface IDeleteDialogData {
  open: boolean;
  currentSelection?: ContainerInfoView[];
}
