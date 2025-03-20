import { ContainerLogsContext } from './ContainerLogsProvider';
import { useContextSelector } from 'use-context-selector';
import { Highlight, themes } from 'prism-react-renderer';
import { useEffect, useRef } from 'react';

const ContainerLogs = () => {
  const logs = useContextSelector(ContainerLogsContext, (v) => v?.logs);
  const isPending = useContextSelector(ContainerLogsContext, (v) => v?.isPending);
  const scrollRef = useRef<HTMLPreElement>(null);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [logs]);

  return (
    <Highlight
      theme={themes.nightOwl}
      code={isPending && logs?.length === 0 ? 'Loading...' : logs!.join('\n')}
      language="tsx">
      {({ style, tokens, getLineProps, getTokenProps }) => (
        <pre
          ref={scrollRef}
          style={style}
          className="bg-card-foreground dark:bg-card p-6! rounded-sm shadow-xs w-full overflow-auto max-w-[1400px] max-h-[600px] scrollbar-thumb-rounded scrollbar-track-rounded scrollbar-thin scrollbar-track-transparent scrollbar-thumb-muted">
          {tokens.map((line, i) => (
            <div key={i} {...getLineProps({ line })} className="table-row">
              <span className="table-cell pr-4 text-xs text-gray-500 text-right select-none">{i + 1}</span>
              {line.map((token, key) => (
                <span key={key} {...getTokenProps({ token })} className="text-sm" />
              ))}
            </div>
          ))}
        </pre>
      )}
    </Highlight>
  );
};

export default ContainerLogs;
