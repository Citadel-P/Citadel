import { useCallback, useEffect, useMemo, useState } from 'react';
import { DiffEditor, Editor, Monaco, type OnMount } from '@monaco-editor/react';
import * as monaco from 'monaco-editor';
import yaml from 'js-yaml';
import * as prettier from 'prettier/standalone';
import * as pluginTypescript from 'prettier/plugins/typescript';
import * as pluginEsTree from 'prettier/plugins/estree';
import * as pluginYaml from 'prettier/plugins/yaml';

import { useLayoutContext } from '@/lib/context/layout-context';
import { useWindowDimensions } from '../hooks';
import { cn } from '../utils';

// --- Configuration Constants ---
const LINE_HEIGHT_PX = 18;
const CONTAINER_PADDING_PX = 30; // Vertical padding buffer
const DEFAULT_MIN_HEIGHT = 56;
const DEFAULT_MAX_HEIGHT = 500;

export type SupportedLanguage =
  | 'yaml'
  | 'json'
  | 'ini'
  | 'shell'
  | 'dockerfile'
  | 'javascript'
  | 'typescript'
  | 'string_list' // Custom language support
  | 'key_value';

// --- Helper: Serialization ---
const serializeData = (data: unknown, format: 'json' | 'yaml') => {
  try {
    if (format === 'yaml') {
      return yaml.dump(data, { noRefs: true });
    }
    return JSON.stringify(data, null, 2);
  } catch {
    return format === 'yaml' ? '# Error serializing YAML' : '// Error serializing JSON';
  }
};

const useDynamicHeight = (
  content: string | undefined,
  minHeight: number = DEFAULT_MIN_HEIGHT,
  maxHeight: number = DEFAULT_MAX_HEIGHT,
) => {
  const { height: windowHeight } = useWindowDimensions();

  return useMemo(() => {
    const lines = content?.split(/\r\n|\r|\n/).length ?? 0;
    const requiredHeight = lines * LINE_HEIGHT_PX + CONTAINER_PADDING_PX;

    const screenLimit = windowHeight > 0 ? Math.floor(windowHeight * 0.5) : maxHeight;
    const effectiveMax = Math.min(screenLimit, maxHeight);

    return Math.max(Math.min(requiredHeight, effectiveMax), minHeight);
  }, [content, windowHeight, minHeight, maxHeight]);
};

const useEditorFormatting = (editor: monaco.editor.IStandaloneCodeEditor | null, language: SupportedLanguage) => {
  useEffect(() => {
    if (!editor) return;

    const isSupported = ['yaml', 'typescript', 'javascript', 'json'].includes(language);
    if (!isSupported) return;

    // Register Command: Alt + Shift + F
    editor.addCommand(monaco.KeyMod.Alt | monaco.KeyMod.Shift | monaco.KeyCode.KeyF, async () => {
      const model = editor.getModel();
      if (!model) return;

      const currentText = editor.getValue();
      const cursorPosition = editor.getPosition();
      const currentOffset = cursorPosition ? model.getOffsetAt(cursorPosition) : 0;

      try {
        const { formatted, cursorOffset } = await prettier.formatWithCursor(currentText, {
          cursorOffset: currentOffset,
          parser: language === 'yaml' ? 'yaml' : 'typescript',
          plugins: language === 'yaml' ? [pluginYaml] : [pluginTypescript, pluginEsTree],
          printWidth: 80,
          tabWidth: 2,
        } as any);

        editor.setValue(formatted);
        editor.setPosition(model.getPositionAt(cursorOffset));
      } catch (error) {
        console.warn('Prettier formatting failed', error);
      }
    });

    return () => {};
  }, [editor, language]);
};

function useThemeEditor() {
  const { theme } = useLayoutContext();
  const currentTheme = theme.mode === 'dark' ? 'vs-dark' : 'custom-light';

  const handleBeforeMount = useCallback((monaco: Monaco) => {
    monaco.editor.defineTheme('custom-light', {
      base: 'vs',
      inherit: true,
      rules: [],
      colors: {
        'editor.background': '#f8fafc',
      },
    });
  }, []);

  return { currentTheme, handleBeforeMount };
}

interface MonacoEditorProps {
  value: string | undefined;
  onValueChange?: (value: string) => void;
  language?: SupportedLanguage;
  filename?: string;
  readOnly?: boolean;
  minHeight?: number;
  className?: string;
}

export const MonacoEditor = ({
  value = '',
  onValueChange,
  language = 'yaml',
  filename,
  readOnly,
  minHeight,
  className,
}: MonacoEditorProps) => {
  const [editorInstance, setEditorInstance] = useState<monaco.editor.IStandaloneCodeEditor | null>(null);
  const { currentTheme, handleBeforeMount } = useThemeEditor();

  // Calculate dynamic height based on line count
  const containerHeight = useDynamicHeight(value, minHeight);

  // Attach formatting logic
  useEditorFormatting(editorInstance, language);

  // Handle Escape key to blur
  useEffect(() => {
    if (!editorInstance) return;
    const domNode = editorInstance.getDomNode();

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        if (document.activeElement instanceof HTMLElement) {
          document.activeElement.blur();
        }
      }
    };

    domNode?.addEventListener('keydown', handleKeyDown);
    return () => domNode?.removeEventListener('keydown', handleKeyDown);
  }, [editorInstance]);

  const handleMount: OnMount = (editor) => {
    setEditorInstance(editor);
  };

  const getPath = (fname?: string) => fname?.split('/').pop();

  return (
    <div className={cn('mx-2 my-1 w-full relative min-w-0', className)} style={{ height: `${containerHeight}px` }}>
      <div className="absolute inset-0">
        <Editor
          language={language}
          value={value}
          theme={currentTheme}
          path={getPath(filename)}
          height="100%"
          width="100%"
          beforeMount={handleBeforeMount}
          onChange={(v) => onValueChange?.(v ?? '')}
          onMount={handleMount}
          options={{
            minimap: { enabled: false },
            scrollBeyondLastLine: false,
            folding: false,
            automaticLayout: true,
            renderValidationDecorations: 'on',
            renderLineHighlightOnlyWhenFocus: true,
            readOnly,
            tabSize: 2,
            detectIndentation: true,
            quickSuggestions: true,
            padding: { top: 15 },
          }}
        />
      </div>
    </div>
  );
};

interface MonacoToArrayStringEditorProps {
  value?: string[];
  language: SupportedLanguage;
  helperText?: string;
  onChange: (v?: string[]) => void;
}

export const MonacoToArrayStringEditor = ({
  value,
  language,
  helperText,
  onChange,
}: MonacoToArrayStringEditorProps) => {
  const cleanHelper = helperText?.trim();

  const generateDisplayContent = useCallback(
    (val: string[] | undefined) => {
      const valueString = (val || []).join('\n');

      if (!cleanHelper) return valueString;

      return valueString ? `${cleanHelper}\n${valueString}` : cleanHelper;
    },
    [cleanHelper],
  );

  const textToArray = (text: string): string[] => {
    return text
      .split('\n')
      .map((line) => line.trim())
      .filter((line) => line.length > 0 && !line.startsWith('#'));
  };

  const [raw, setRaw] = useState(() => generateDisplayContent(value));

  useEffect(() => {
    const expected = generateDisplayContent(value);

    if (raw !== expected) {
      const currentData = textToArray(raw).join('\n');
      const expectedData = (value || []).join('\n');

      if (currentData !== expectedData || !raw.startsWith(cleanHelper ?? '# key = value')) {
        setRaw(expected);
      }
    }
  }, [value, cleanHelper, raw, generateDisplayContent]);

  return (
    <MonacoEditor
      language={language}
      value={raw}
      onValueChange={(text) => {
        const currentText = text || '';
        setRaw(currentText);
        onChange(textToArray(currentText));
      }}
    />
  );
};

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
  const { currentTheme, handleBeforeMount } = useThemeEditor();

  const origText = serializeData(original, format);
  const modText = serializeData(modified, format);

  const lineCount = Math.max(origText.split(/\r\n|\r|\n/).length, modText.split(/\r\n|\r|\n/).length);

  useEffect(() => {
    if (!editor) return;

    const container = editor.getContainerDomNode();
    const height = Math.max(Math.min(lineCount * 18 + 40, DEFAULT_MAX_HEIGHT), DEFAULT_MIN_HEIGHT);

    container.style.height = `${height}px`;

    editor.layout();
  }, [editor, lineCount]);

  return (
    <div className="w-full min-w-0 mx-1">
      <DiffEditor
        original={origText}
        modified={modText}
        language={format}
        beforeMount={handleBeforeMount}
        theme={currentTheme}
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
