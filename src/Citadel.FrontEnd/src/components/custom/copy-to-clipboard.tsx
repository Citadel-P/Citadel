import { useCopyToClipboard } from '@/hooks/useCopyToClipboard';
import { cn } from '@/lib/utils';
import { CheckCheck, Clipboard } from 'lucide-react';

export const CopyToClipboard = ({
  textToCopy,
  transform,
  groupClassName,
  textClassName,
}: {
  textToCopy: string;
  transform?: (text: string) => string;
  groupClassName?: string;
  textClassName?: string;
}) => {
  const [copyCmd, setCopyCmd] = useCopyToClipboard(3000);

  return (
    <div className={cn('flex gap-0.5 items-center group', groupClassName)}>
      <div className={cn('break-all md:break-normal', textClassName)}>
        {transform ? transform(textToCopy) : textToCopy}
      </div>
      <button
        aria-label="Copy to clipboard"
        className={cn(
          'rounded-full ml-1 px-1.5 py-1.5 hover:bg-foreground/10 text-sm font-semibold',
          copyCmd ? 'visible' : 'invisible group-hover:visible group-hover/rowid:visible focus-visible:visible',
        )}
        onClick={() => setCopyCmd(textToCopy ?? '')}>
        {copyCmd ? <CheckCheck className="w-3 h-3 text-green-500" /> : <Clipboard className="w-3 h-3 text-primary" />}
      </button>
    </div>
  );
};
