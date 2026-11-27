import type { ReactNode } from 'react';
import { Pencil } from 'lucide-react';
import { Button } from '@/components/ui/button';

export function InlineTitleLabel({
  value,
  status,
  onEdit,
}: {
  value: string;
  status?: ReactNode;
  onEdit?: () => void;
}) {
  return (
    <div className="group/title flex w-fit min-w-0 max-w-full flex-wrap items-center gap-x-2 gap-y-2">
      <h1 className="min-w-0 max-w-full text-2xl font-semibold tracking-tight [overflow-wrap:anywhere]">
        {!onEdit ? (
          value
        ) : (
          <button
            type="button"
            onClick={onEdit}
            className="max-w-full cursor-text rounded-sm text-left focus-visible:outline-2 focus-visible:outline-ring">
            {value}
          </button>
        )}
      </h1>

      <div
        className={
          !onEdit
            ? 'flex shrink-0 items-center'
            : 'relative flex shrink-0 items-center pl-0 transition-[padding] duration-150 ease-out group-hover/title:pl-8 group-focus-within/title:pl-8 motion-reduce:transition-none'
        }>
        {onEdit && (
          <Button
            type="button"
            size="icon-sm"
            variant="ghost"
            aria-label="Edit name"
            onClick={onEdit}
            className="pointer-events-none absolute left-0 size-7 -translate-x-1 opacity-0 transition-[opacity,transform] duration-150 ease-out group-hover/title:pointer-events-auto group-hover/title:translate-x-0 group-hover/title:opacity-100 group-focus-within/title:pointer-events-auto group-focus-within/title:translate-x-0 group-focus-within/title:opacity-100 motion-reduce:transition-none">
            <Pencil className="size-3.5" />
          </Button>
        )}
        {status}
      </div>
    </div>
  );
}
