import { ChevronDown, ChevronUp, Copy, Database, FileSliders } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
  DropdownMenuSub,
  DropdownMenuSubTrigger,
  DropdownMenuSubContent,
  DropdownMenuPortal,
} from '@/components/ui/dropdown-menu';

export type DuplicateMode = 'configuration' | 'data';

type DuplicateMenuProps = {
  onSelect: (mode: DuplicateMode) => void;
  disabled?: boolean;
  dataDisabledReason?: string;
  submenu?: boolean;
  side?: 'top' | 'bottom';
};

export function DuplicateMenu({
  onSelect,
  disabled = false,
  dataDisabledReason,
  submenu = false,
  side = 'bottom',
}: DuplicateMenuProps) {
  const Chevron = side === 'top' ? ChevronUp : ChevronDown;
  const items = <DuplicateMenuItems onSelect={onSelect} disabled={disabled} dataDisabledReason={dataDisabledReason} />;
  const contentClassName = 'w-[22rem] max-w-[calc(100vw-2rem)] p-1.5';
  if (submenu) {
    return (
      <DropdownMenuSub>
        <DropdownMenuSubTrigger disabled={disabled} className="gap-2 px-3 py-2 text-xs font-semibold">
          <Copy className="size-4" aria-hidden="true" />
          Duplicate
        </DropdownMenuSubTrigger>
        <DropdownMenuPortal>
          <DropdownMenuSubContent sideOffset={6} collisionPadding={12} className={contentClassName}>
            {items}
          </DropdownMenuSubContent>
        </DropdownMenuPortal>
      </DropdownMenuSub>
    );
  }
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild disabled={disabled}>
        <Button type="button" variant="outline" size="sm" disabled={disabled} className="group">
          <Copy className="size-3.5" aria-hidden="true" />
          Duplicate
          <Chevron
            className="ml-1 size-3.5 text-muted-foreground transition-transform group-data-[state=open]:rotate-180 motion-reduce:transition-none"
            aria-hidden="true"
          />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" side={side} sideOffset={6} collisionPadding={12} className={contentClassName}>
        {items}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

function DuplicateMenuItems({ onSelect, disabled, dataDisabledReason }: DuplicateMenuProps) {
  const dataUnavailable = Boolean(dataDisabledReason);
  return (
    <>
      <DropdownMenuItem
        disabled={disabled}
        onSelect={() => onSelect('configuration')}
        className="items-start gap-3 rounded-md p-3 transition-colors motion-reduce:transition-none">
        <span className="flex size-9 shrink-0 items-center justify-center rounded-md border bg-background text-primary">
          <FileSliders className="size-4 text-primary" aria-hidden="true" />
        </span>
        <span className="min-w-0 space-y-1">
          <span className="block font-medium">Configuration only</span>
          <span className="block text-xs leading-relaxed text-muted-foreground">
            Copy settings without copying existing data.
          </span>
        </span>
      </DropdownMenuItem>
      <DropdownMenuSeparator className="mx-3 my-1" />
      <DropdownMenuItem
        disabled={disabled || dataUnavailable}
        onSelect={() => onSelect('data')}
        className="items-start gap-3 rounded-md p-3 transition-colors data-[disabled]:text-muted-foreground data-[disabled]:opacity-100 motion-reduce:transition-none">
        <span className="flex size-9 shrink-0 items-center justify-center rounded-md border bg-background">
          <Database
            className={`size-4 ${dataUnavailable ? 'text-muted-foreground' : 'text-primary'}`}
            aria-hidden="true"
          />
        </span>
        <span className="min-w-0 space-y-1">
          <span className="block font-medium">Configuration + data</span>
          <span className="block break-words text-xs leading-relaxed text-muted-foreground" aria-live="polite">
            {dataDisabledReason || 'Copy current data or restore backups into new volumes.'}
          </span>
        </span>
      </DropdownMenuItem>
    </>
  );
}
