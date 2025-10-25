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
    <div className={`flex gap-0.5 items-center ${groupClassName ? groupClassName : 'group'}`}>
      <div className={cn('break-all md:break-normal', textClassName)}>
        {transform ? transform(textToCopy) : textToCopy}
      </div>
      <button
        className={`rounded-full ml-1 px-1.5 py-1.5 hover:bg-foreground/10 text-sm font-semibold
          ${copyCmd ? 'visible' : `invisible group-hover${groupClassName ? '/' + groupClassName : ''}:visible`}`}
        onClick={() => setCopyCmd(textToCopy ?? '')}>
        {copyCmd ? <CheckCheck className="w-3 h-3 text-green-500" /> : <Clipboard className="w-3 h-3 text-primary" />}
      </button>
    </div>
  );
};
