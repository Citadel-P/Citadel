import { CSSProperties, useEffect, useMemo, useRef, useState } from 'react';
import { Highlight, themes } from 'prism-react-renderer';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { Clipboard, CheckCheck } from 'lucide-react';
import { useCopyToClipboard } from '@/hooks/useCopyToClipboard';
import { useLayoutContext } from '@/lib/context/layout-context';

export type CodeHighlightProps = {
  code: string;
  language?: string;
  showLineNumbers?: boolean;
  className?: string; // applied to <pre>
  style?: CSSProperties; // merged into <pre> style
  lineNumberClassName?: string;
  lineContentClassName?: string;
  lineWrapperClassName?: string;
  autoScroll?: boolean;
  showCopyButton?: boolean;
  copyLabel?: string;
};

export function CodeHighlight({
  code,
  language = 'tsx',
  showLineNumbers = false,
  className = 'p-4 max-h-[600px] rounded-sm border shadow-xs inline-block w-full overflow-auto bg-transparent ',
  style,
  lineNumberClassName = 'table-cell pr-4 text-sm text-gray-500 text-right select-none',
  lineContentClassName = 'text-xs',
  lineWrapperClassName = 'table-row',
  autoScroll = false,
  showCopyButton = false,
  copyLabel = 'Copy',
}: CodeHighlightProps) {
  const scrollRef = useRef<HTMLPreElement>(null);
  const [copied, copyToClipboard] = useCopyToClipboard(3000);
  const [codeTheme, setCodeTheme] = useState(themes.vsLight);
  const { theme } = useLayoutContext();

  useEffect(() => {
    setCodeTheme(theme.mode === 'dark' ? themes.vsDark : themes.vsLight);
  }, [theme.mode]);

  const safeCode = useMemo(() => (code?.length ? code : ''), [code]);

  useEffect(() => {
    if (!autoScroll) return;
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [autoScroll, safeCode]);

  return (
    <Highlight theme={codeTheme} code={safeCode} language={language}>
      {({ style: hlStyle, tokens, getLineProps, getTokenProps }) => (
        <pre
          ref={scrollRef}
          style={{ ...hlStyle, ...style }}
          className={`${className} ${showCopyButton ? 'relative' : ''}`}>
          {tokens.map((line, i) => (
            <div key={i} {...getLineProps({ line })} className={lineWrapperClassName}>
              {showLineNumbers && <span className={lineNumberClassName}>{i + 1}</span>}
              {line.map((token, key) => (
                <span key={key} {...getTokenProps({ token })} className={lineContentClassName} />
              ))}
            </div>
          ))}
          {showCopyButton && (
            <TooltipProvider delayDuration={200}>
              <Tooltip>
                <TooltipTrigger asChild>
                  <button
                    className="rounded-md px-1.5 py-1.5 absolute top-2 right-2 text-sm font-semibold"
                    onClick={() => copyToClipboard(safeCode)}
                    aria-label={copyLabel}>
                    {copied ? <CheckCheck className="w-4 h-4 text-green-500" /> : <Clipboard className="w-4 h-4" />}
                  </button>
                </TooltipTrigger>
                <TooltipContent>{copyLabel}</TooltipContent>
              </Tooltip>
            </TooltipProvider>
          )}
        </pre>
      )}
    </Highlight>
  );
}

export default CodeHighlight;
