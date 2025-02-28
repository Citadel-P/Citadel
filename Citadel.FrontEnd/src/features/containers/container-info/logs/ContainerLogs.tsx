import { ContainerLogsContext } from './ContainerLogsProvider';
import { useContextSelector } from 'use-context-selector';
import { useEffect, useState } from 'react';
import { Highlight, themes } from 'prism-react-renderer';

const ContainerLogs = () => {
  const logs = useContextSelector(ContainerLogsContext, (v) => v?.logs);
  const isPending = useContextSelector(ContainerLogsContext, (v) => v?.isPending);
  const [logsBuffer, setLogsBuffer] = useState(logs);

  useEffect(() => {
    setLogsBuffer((currentLogs) => [...(currentLogs ?? []), ...(logs ?? [])]);
  }, [logs]);

  return (
    <Highlight theme={themes.nightOwl} code={isPending ? 'Loading...' : logsBuffer!.join('\n')} language="tsx">
      {({ style, tokens, getLineProps, getTokenProps }) => (
        <pre
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
