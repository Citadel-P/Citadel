// check https://github.com/radix-ui/primitives/issues/1836#issuecomment-2051812652
import { useRef, useState } from 'react';

export function useDialog(): ICustomDialog {
  const [isOpen, setIsOpen] = useState(false);
  const triggerRef = useRef(null);

  function trigger() {
    setIsOpen(true);
  }

  function dismiss() {
    setIsOpen(false);
    triggerRef.current?.focus();
  }

  return {
    triggerProps: {
      ref: triggerRef,
      onClick: trigger,
    },
    dialogProps: {
      open: isOpen,
      onOpenChange: (open: boolean) => {
        if (open) trigger();
        else dismiss();
      },
    },
    trigger,
    dismiss,
  };
}

export interface ICustomDialog {
  triggerProps: ITriggerProps;
  dialogProps: IDialogProps;
  trigger: () => void;
  dismiss: () => void;
}
interface ITriggerProps {
  ref: React.MutableRefObject<undefined>;
  onClick: () => void;
}
interface IDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}
