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
    <div className={cn('flex gap-0.5 items-center group min-w-0', groupClassName)}>
      <div className={cn('truncate whitespace-nowrap overflow-hidden', textClassName)} title={textToCopy}>
        {transform ? transform(textToCopy) : textToCopy}
      </div>

      <button
        aria-label="Copy to clipboard"
        className={cn(
          'shrink-0 rounded-full px-1.5 py-1.5 hover:bg-foreground/10',
          copyCmd ? 'visible' : 'invisible group-hover:visible group-hover/rowid:visible focus-visible:visible',
        )}
        onClick={() => setCopyCmd(textToCopy)}>
        {copyCmd ? <CheckCheck className="w-3 h-3 text-green-500" /> : <Clipboard className="w-3 h-3 text-primary" />}
      </button>
    </div>
  );
};
