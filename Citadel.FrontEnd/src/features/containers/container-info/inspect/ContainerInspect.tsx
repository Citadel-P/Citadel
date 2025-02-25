import { useParams } from "react-router";
import { usGETContainerInspect } from "./hooks/useGETContainerInspect";
import { Highlight, themes } from 'prism-react-renderer';

const ContainerInspect = () => {
    const { containerId } = useParams();
    const { data, isSuccess, isLoading } = usGETContainerInspect(containerId);
    let code = "";

    if (isSuccess && data?.data) {
        code = JSON.stringify(data.data, null, 2);
    }

    if (isLoading) {
        code = "Loading...";
    }
    return (
    <Highlight theme={themes.nightOwl} code={code} language="tsx">
      {({ className, style, tokens, getLineProps, getTokenProps }) => (
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
}

export default ContainerInspect;