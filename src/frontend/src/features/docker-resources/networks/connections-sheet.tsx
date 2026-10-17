import { useState, type ReactNode } from 'react';
import { Maximize2, Minimize2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Sheet, SheetContent, SheetDescription, SheetFooter, SheetHeader, SheetTitle } from '@/components/ui/sheet';
import { cn } from '@/lib/utils';

export function ConnectionsSheet({
  title,
  description,
  icon,
  onClose,
  onCloseAutoFocus,
  children,
  footer,
}: {
  title: string;
  description: ReactNode;
  icon: ReactNode;
  onClose: () => void;
  onCloseAutoFocus?: (event: Event) => void;
  children: (maximized: boolean) => ReactNode;
  footer?: ReactNode;
}) {
  const [maximized, setMaximized] = useState(false);
  return (
    <Sheet
      open
      onOpenChange={(open) => {
        if (!open) onClose();
      }}>
      <SheetContent
        className={cn('w-full gap-0 sm:max-w-[720px]', maximized && 'sm:max-w-none')}
        onCloseAutoFocus={onCloseAutoFocus}
        onEscapeKeyDown={(event) => {
          if (maximized) {
            event.preventDefault();
            setMaximized(false);
          }
        }}>
        <SheetHeader className="shrink-0 border-b px-5 py-4 pr-24">
          <div className="flex min-w-0 items-center gap-3">
            <div className="flex size-10 shrink-0 items-center justify-center">{icon}</div>
            <div className="min-w-0">
              <SheetTitle className="truncate" title={title}>
                {title}
              </SheetTitle>
              <SheetDescription className="mt-1">{description}</SheetDescription>
            </div>
          </div>
        </SheetHeader>
        <Button
          variant="ghost"
          size="icon"
          className="absolute top-2.5 right-10 size-8"
          aria-label={maximized ? 'Restore network sheet' : 'Maximize network sheet'}
          aria-pressed={maximized}
          onClick={() => setMaximized((value) => !value)}>
          {maximized ? <Minimize2 className="size-4" /> : <Maximize2 className="size-4" />}
        </Button>
        {children(maximized)}
        {footer && <SheetFooter className="shrink-0 border-t bg-background px-5 py-4">{footer}</SheetFooter>}
      </SheetContent>
    </Sheet>
  );
}
