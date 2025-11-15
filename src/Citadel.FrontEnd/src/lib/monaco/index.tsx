import { DiffEditor } from '@monaco-editor/react';
import { useEffect, useState } from 'react';
import * as monaco from 'monaco-editor';
import yaml from 'js-yaml';
import { useLayoutContext } from '@/lib/context/layout-context';

const MIN_HEIGHT = 120;
const MAX_HEIGHT = 500;

function toYaml(v: unknown) {
  try {
    return yaml.dump(v, { noRefs: true });
  } catch {
    return '# YAML serialization error';
  }
}

function toJson(v: unknown) {
  try {
    return JSON.stringify(v, null, 2);
  } catch {
    return '// JSON serialization error';
  }
}

export function MonacoDiff({
  original,
  modified,
  format,
}: {
  original: unknown;
  modified: unknown;
  format: 'json' | 'yaml';
}) {
  const [editor, setEditor] = useState<monaco.editor.IStandaloneDiffEditor | null>(null);
  const { theme } = useLayoutContext();
  const origText = format === 'yaml' ? toYaml(original) : toJson(original);
  const modText = format === 'yaml' ? toYaml(modified) : toJson(modified);

  const lineCount = Math.max(origText.split(/\r\n|\r|\n/).length, modText.split(/\r\n|\r|\n/).length);

  useEffect(() => {
    if (!editor) return;

    const container = editor.getContainerDomNode();
    const height = Math.max(Math.min(lineCount * 18 + 40, MAX_HEIGHT), MIN_HEIGHT);

    container.style.height = `${height}px`;

    editor.layout();
  }, [editor, lineCount]);

  return (
    <div className="w-full min-w-0 mx-1">
      <DiffEditor
        original={origText}
        modified={modText}
        language={format}
        theme={`vs-${theme.mode}`}
        keepCurrentModifiedModel={true}
        keepCurrentOriginalModel={true}
        options={{
          automaticLayout: true,
          renderSideBySide: true,
          scrollBeyondLastLine: false,
          minimap: { enabled: false },
          hideUnchangedRegions: { enabled: true },
          readOnly: true,
        }}
        onMount={(editorInstance) => {
          setEditor(editorInstance);
          requestAnimationFrame(() => {
            editorInstance.layout();
          });
        }}
      />
    </div>
  );
}
