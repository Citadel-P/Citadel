import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { DiffEditor, Editor, Monaco, type OnMount } from '@monaco-editor/react';
import * as monaco from 'monaco-editor';

import { useLayoutContext } from '@/lib/context/layout-context';
import { useLocalStorage, useWindowDimensions } from '../hooks';
import { cn, serializeData } from '../utils';
import { ButtonGroup } from '@/components/ui/button-group';
import { Button } from '@/components/ui/button';
import { Columns2, Rows4 } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';

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

type DiffFormat = 'json' | 'yaml';
type DiffLayout = 'side-by-side' | 'inline';

let prettierLoader: Promise<{
  formatWithCursor: (source: string, options: any) => Promise<{ formatted: string; cursorOffset: number }>;
  pluginTypescript: any;
  pluginEsTree: any;
  pluginYaml: any;
}> | null = null;

const loadPrettierFormatter = () => {
  if (!prettierLoader) {
    prettierLoader = Promise.all([
      import('prettier/standalone'),
      import('prettier/plugins/typescript'),
      import('prettier/plugins/estree'),
      import('prettier/plugins/yaml'),
    ]).then(([prettier, pluginTypescript, pluginEsTree, pluginYaml]) => ({
      formatWithCursor: prettier.formatWithCursor,
      pluginTypescript: pluginTypescript.default,
      pluginEsTree: pluginEsTree.default,
      pluginYaml: pluginYaml.default,
    }));
  }

  return prettierLoader;
};
// --- Helper: Serialization ---

const useDynamicHeight = (
  content: string | undefined,
  minHeight: number = DEFAULT_MIN_HEIGHT,
  maxHeight: number = DEFAULT_MAX_HEIGHT,
) => {
  const { height: windowHeight } = useWindowDimensions();

  return useMemo(() => {
    const lines = countLines(content ?? '');
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

    let active = true;

    const formatCurrentDocument = async () => {
      const model = editor.getModel();
      if (!model) return;

      const currentText = editor.getValue();
      const cursorPosition = editor.getPosition();
      const currentOffset = cursorPosition ? model.getOffsetAt(cursorPosition) : 0;

      try {
        const { formatWithCursor, pluginTypescript, pluginEsTree, pluginYaml } = await loadPrettierFormatter();

        const { formatted, cursorOffset } = await formatWithCursor(currentText, {
          cursorOffset: currentOffset,
          parser: language === 'yaml' ? 'yaml' : 'typescript',
          plugins: language === 'yaml' ? [pluginYaml] : [pluginTypescript, pluginEsTree],
          printWidth: 80,
          tabWidth: 2,
        } as any);

        if (!active) return;

        editor.pushUndoStop();
        editor.executeEdits('citadel.format', [
          { range: model.getFullModelRange(), text: formatted, forceMoveMarkers: true },
        ]);
        editor.pushUndoStop();
        editor.setPosition(model.getPositionAt(cursorOffset));
      } catch (error) {
        console.warn('Prettier formatting failed', error);
      }
    };

    const actionDisposable = editor.addAction({
      id: `citadel.format.${language}`,
      label: 'Format Document',
      keybindings: [monaco.KeyMod.Alt | monaco.KeyMod.Shift | monaco.KeyCode.KeyF],
      run: async () => {
        await formatCurrentDocument();
      },
    });

    return () => {
      active = false;
      actionDisposable.dispose();
    };
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
  folding?: boolean;
  minimap?: boolean;
  fontSize?: number;
  title?: string;
}

export const MonacoEditor = ({
  value = '',
  onValueChange,
  language = 'yaml',
  filename,
  readOnly,
  minHeight,
  className,
  folding = false,
  minimap = false,
  fontSize = 13,
  title,
}: MonacoEditorProps) => {
  const [editorInstance, setEditorInstance] = useState<monaco.editor.IStandaloneCodeEditor | null>(null);
  const lastEditorValueRef = useRef(value);
  const { currentTheme, handleBeforeMount } = useThemeEditor();

  // Calculate dynamic height based on line count
  const containerHeight = useDynamicHeight(value, minHeight);

  // Attach formatting logic
  useEditorFormatting(editorInstance, language);

  // Handle Escape key to blur
  useEffect(() => {
    if (!editorInstance) return;
    const keydownDisposable = editorInstance.onKeyDown((event) => {
      if (event.keyCode !== monaco.KeyCode.Escape) return;

      if (document.activeElement instanceof HTMLElement) {
        document.activeElement.blur();
      }
    });

    return () => keydownDisposable.dispose();
  }, [editorInstance]);

  useEffect(() => {
    return () => {
      if (editorInstance) {
        editorInstance.dispose();
      }
    };
  }, [editorInstance]);

  const handleMount: OnMount = useCallback((editor) => {
    lastEditorValueRef.current = editor.getValue();
    setEditorInstance(editor);
  }, []);

  useEffect(() => {
    if (!editorInstance) return;
    if (value === lastEditorValueRef.current) return;
    if (value === editorInstance.getValue()) {
      lastEditorValueRef.current = value;
      return;
    }

    const model = editorInstance.getModel();
    if (!model) return;

    const position = editorInstance.getPosition();
    const selections = editorInstance.getSelections();
    const scrollTop = editorInstance.getScrollTop();
    const scrollLeft = editorInstance.getScrollLeft();

    model.setValue(value);

    if (selections) {
      editorInstance.setSelections(selections);
    } else if (position) {
      editorInstance.setPosition(model.validatePosition(position));
    }
    editorInstance.setScrollTop(scrollTop);
    editorInstance.setScrollLeft(scrollLeft);

    lastEditorValueRef.current = value;
  }, [editorInstance, value]);

  const editorPath = useMemo(() => {
    if (!filename) return undefined;
    const basename = filename.split(/[\\/]/).pop();
    return basename ? `file:///${basename}` : undefined;
  }, [filename]);
  const handleEditorChange = useCallback(
    (nextValue: string | undefined) => {
      const next = nextValue ?? '';
      lastEditorValueRef.current = next;
      onValueChange?.(next);
    },
    [onValueChange],
  );
  const editorOptions = useMemo<monaco.editor.IStandaloneEditorConstructionOptions>(
    () => ({
      minimap: { enabled: minimap },
      scrollBeyondLastLine: false,
      folding: folding,
      links: true,
      automaticLayout: true,
      occurrencesHighlight: 'singleFile',
      renderValidationDecorations: 'on',
      renderLineHighlightOnlyWhenFocus: true,
      readOnly,
      tabSize: 2,
      detectIndentation: true,
      quickSuggestions: true,
      padding: { top: 15 },
      fontSize: fontSize,
      lineHeight: 20,
      fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
      fontLigatures: true,
      letterSpacing: 0.5,
    }),
    [folding, fontSize, minimap, readOnly],
  );
  const containerStyle = useMemo(() => ({ height: `${containerHeight}px` }), [containerHeight]);

  return (
    <div className={cn('mx-2 my-1 w-full relative min-w-0', className)} style={containerStyle}>
      <div className="flex flex-col gap-2 absolute inset-0">
        <span className="text-sm font-medium text-foreground">{title}</span>

        <Editor
          language={language}
          defaultValue={value}
          theme={currentTheme}
          path={editorPath}
          height="100%"
          width="100%"
          beforeMount={handleBeforeMount}
          onChange={handleEditorChange}
          onMount={handleMount}
          options={editorOptions}
        />
      </div>
    </div>
  );
};

interface MonacoToArrayEditorProps {
  value?: string[];
  language: SupportedLanguage;
  helperText?: string;
  onChange: (v?: string[]) => void;
}

export const MonacoToArrayEditor = ({ value, language, helperText, onChange }: MonacoToArrayEditorProps) => {
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

  const [raw, setRaw] = useState(() => {
    const initial = generateDisplayContent(value);
    return initial;
  });
  const rawRef = useRef(raw);
  const prevArrayValueRef = useRef<string[] | undefined>(value);

  useEffect(() => {
    rawRef.current = raw;
  }, [raw]);

  useEffect(() => {
    const prev = prevArrayValueRef.current || [];
    const next = value || [];

    const prevNorm = (prev || []).join('\n');
    const nextNorm = (next || []).join('\n');

    prevArrayValueRef.current = value;

    if (prevNorm === nextNorm) return;

    const expected = generateDisplayContent(value);
    const currentData = textToArray(rawRef.current).join('\n');

    if (currentData !== nextNorm) {
      rawRef.current = expected;
      setRaw(expected);
    }
  }, [value, cleanHelper, generateDisplayContent]);

  return (
    <MonacoEditor
      language={language}
      value={raw}
      onValueChange={(text) => {
        const currentText = text || '';
        rawRef.current = currentText;
        setRaw(currentText);
        onChange(textToArray(currentText));
      }}
    />
  );
};

interface MonacoToDictionaryEditorProps {
  value?: Record<string, string | null>;
  language?: SupportedLanguage;
  helperText?: string;
  onChange: (v?: Record<string, string>) => void;
}

export const MonacoToDictionaryEditor = ({
  value,
  language = 'ini',
  helperText,
  onChange,
}: MonacoToDictionaryEditorProps) => {
  const cleanHelper = helperText?.trim();
  const rawRef = useRef<string>('');

  const generateDisplayContent = useCallback(
    (val: Record<string, string | null> | undefined) => {
      const valueString = Object.entries(val || {})
        .filter(([_, v]) => v !== null && v !== undefined)
        .map(([k, v]) => `${k}=${v}`)
        .join('\n');

      if (!cleanHelper) return valueString;
      return valueString ? `${cleanHelper}\n${valueString}` : cleanHelper;
    },
    [cleanHelper],
  );

  const textToDictionary = (text: string): Record<string, string> => {
    const result: Record<string, string> = {};
    text.split('\n').forEach((line) => {
      const trimmed = line.trim();
      if (trimmed.length === 0 || trimmed.startsWith('#')) return;

      const separatorIndex = trimmed.indexOf('=');
      if (separatorIndex > 0) {
        const key = trimmed.slice(0, separatorIndex).trim();
        const val = trimmed.slice(separatorIndex + 1).trim();
        if (key) {
          result[key] = val;
        }
      }
    });
    return result;
  };

  const [raw, setRaw] = useState(() => generateDisplayContent(value));

  useEffect(() => {
    rawRef.current = raw;
  }, [raw]);

  const prevValueRef = useRef<Record<string, string | null> | undefined>(value);
  const dictionaryEquals = (left: Record<string, string>, right: Record<string, string>) => {
    const leftEntries = Object.entries(left);
    const rightEntries = Object.entries(right);
    if (leftEntries.length !== rightEntries.length) return false;

    for (const [key, leftValue] of leftEntries) {
      if (!Object.prototype.hasOwnProperty.call(right, key)) return false;
      if (right[key] !== leftValue) return false;
    }
    return true;
  };

  // Sync from props only when the prop value is semantically different
  useEffect(() => {
    const prev = prevValueRef.current || {};
    const next = value || {};

    const normalise = (obj: Record<string, any>) =>
      Object.fromEntries(Object.entries(obj).filter(([_, v]) => v !== null && v !== undefined));

    const prevNorm = normalise(prev as Record<string, any>);
    const nextNorm = normalise(next as Record<string, any>);

    const valueChangedExternally = !dictionaryEquals(
      prevNorm as Record<string, string>,
      nextNorm as Record<string, string>,
    );

    prevValueRef.current = value;

    if (!valueChangedExternally) return;

    const expected = generateDisplayContent(value);
    const currentRaw = rawRef.current;
    const currentData = textToDictionary(currentRaw);

    const expectedData = nextNorm as Record<string, string>;
    const isDataDifferent = !dictionaryEquals(currentData, expectedData);

    if (isDataDifferent) {
      rawRef.current = expected;
      setRaw(expected);
    }
  }, [value, cleanHelper, generateDisplayContent]);

  return (
    <MonacoEditor
      language={language}
      value={raw}
      onValueChange={(text) => {
        const currentText = text || '';
        // Event handler — ref writes are allowed here
        rawRef.current = currentText;
        setRaw(currentText);

        const newDict: Record<string, string | null> = textToDictionary(currentText);

        if (value) {
          for (const oldKey of Object.keys(value)) {
            const wasNotNull = value[oldKey] !== null && value[oldKey] !== undefined;
            const isMissing = !Object.prototype.hasOwnProperty.call(newDict, oldKey);
            if (wasNotNull && isMissing) {
              newDict[oldKey] = undefined as any;
            }
          }
        }

        onChange(newDict as any);
      }}
    />
  );
};

export function MonacoDiff({
  original,
  modified,
  format,
  title,
}: {
  original: unknown;
  modified: unknown;
  format: DiffFormat;
  title?: string;
}) {
  const [editor, setEditor] = useState<monaco.editor.IStandaloneDiffEditor | null>(null);
  const layoutFrameRef = useRef<number | null>(null);
  const mountFrameRef = useRef<number | null>(null);
  const [layout, setLayout] = useLocalStorage<DiffLayout>('monaco-diff-layout', 'side-by-side');
  const { currentTheme, handleBeforeMount } = useThemeEditor();

  const originalText = useMemo(() => serializeData(original, format), [original, format]);
  const modifiedText = useMemo(() => serializeData(modified, format), [modified, format]);

  const maxLineCount = useMemo(() => {
    const originalLines = countLines(originalText);
    const modifiedLines = countLines(modifiedText);
    return Math.max(originalLines, modifiedLines);
  }, [originalText, modifiedText]);

  useEffect(() => {
    if (!editor) return;

    const container = editor.getContainerDomNode();
    const height = clamp(maxLineCount * 18 + 40, DEFAULT_MIN_HEIGHT, DEFAULT_MAX_HEIGHT);
    if (layoutFrameRef.current !== null) {
      cancelAnimationFrame(layoutFrameRef.current);
    }
    layoutFrameRef.current = requestAnimationFrame(() => {
      container.style.height = `${height}px`;
      editor.layout();
      layoutFrameRef.current = null;
    });

    return () => {
      if (layoutFrameRef.current !== null) {
        cancelAnimationFrame(layoutFrameRef.current);
        layoutFrameRef.current = null;
      }
      container.style.height = '';
    };
  }, [editor, maxLineCount]);

  useEffect(() => {
    return () => {
      if (mountFrameRef.current !== null) {
        cancelAnimationFrame(mountFrameRef.current);
      }
      if (layoutFrameRef.current !== null) {
        cancelAnimationFrame(layoutFrameRef.current);
      }
    };
  }, []);

  return (
    <div className="flex flex-col gap-2 w-full min-w-0">
      <DiffEditorHeader layout={layout} onChange={setLayout} title={title} />

      <DiffEditor
        original={originalText}
        modified={modifiedText}
        language={format}
        beforeMount={handleBeforeMount}
        theme={currentTheme}
        options={{
          automaticLayout: true,
          renderSideBySide: layout === 'side-by-side',
          scrollBeyondLastLine: false,
          minimap: { enabled: false },
          hideUnchangedRegions: { enabled: true },
          readOnly: true,
        }}
        onMount={(instance) => {
          setEditor(instance);
          if (mountFrameRef.current !== null) {
            cancelAnimationFrame(mountFrameRef.current);
          }
          mountFrameRef.current = requestAnimationFrame(() => {
            if (!instance.getContainerDomNode().isConnected) return;
            instance.layout();
            mountFrameRef.current = null;
          });
        }}
      />
    </div>
  );
}

function DiffEditorHeader({
  layout,
  title,
  onChange,
}: {
  layout: DiffLayout;
  title?: string;
  onChange: (layout: DiffLayout) => void;
}) {
  const tooltip = layout === 'side-by-side' ? 'Switch to inline diff' : 'Switch to side-by-side diff';

  return (
    <TooltipProvider delayDuration={200}>
      <Tooltip>
        <div className="flex items-center justify-between w-full gap-2">
          <span className="text-sm font-medium text-foreground truncate">{title}</span>
          <TooltipTrigger asChild>
            <ButtonGroup aria-label="Diff layout">
              <Button
                variant={layout === 'inline' ? 'secondary' : 'outline'}
                size="icon-sm"
                className="rounded-none"
                onClick={() => onChange('inline')}
                disabled={layout === 'inline'}>
                <Rows4 className="size-3.5" />
              </Button>

              <Button
                variant={layout === 'side-by-side' ? 'secondary' : 'outline'}
                size="icon-sm"
                className="rounded-none"
                onClick={() => onChange('side-by-side')}
                disabled={layout === 'side-by-side'}>
                <Columns2 className="size-3.5" />
              </Button>
            </ButtonGroup>
          </TooltipTrigger>
        </div>

        <TooltipContent side="top">{tooltip}</TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
function countLines(text: string) {
  return text.split(/\r\n|\r|\n/).length;
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}
