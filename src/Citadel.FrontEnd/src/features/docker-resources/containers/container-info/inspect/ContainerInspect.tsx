import { useParams } from 'react-router';
import { Highlight, themes } from 'prism-react-renderer';
import { useMemo } from 'react';
import { useRead } from '@/lib/hooks';

const ContainerInspect = () => {
  const { containerId } = useParams();
  const { data, isSuccess, isLoading } = useRead('inspectContainer', { id: containerId });

  // Memoize the code to avoid recalculating it on every render
  const code = useMemo(() => {
    if (isLoading) return 'Loading...';
    if (isSuccess && data?.data) return JSON.stringify(data.data, null, 2);
    return '';
  }, [isLoading, isSuccess, data]);

  // Define styles for the <pre> element
  const preClassName =
    'bg-card-foreground dark:bg-card p-6 rounded-sm shadow-xs w-full overflow-auto max-w-[1400px] max-h-[650px]';

  return (
    <Highlight theme={themes.nightOwl} code={code} language="tsx">
      {({ style, tokens, getLineProps, getTokenProps }) => (
        <pre style={style} className={preClassName}>
          {tokens.map((line, i) => (
            <div key={i} {...getLineProps({ line })} className="table-row">
              {/* Line number */}
              <span className="table-cell pr-4 text-xs text-gray-500 text-right select-none">{i + 1}</span>
              {/* Line content */}
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

export default ContainerInspect;
