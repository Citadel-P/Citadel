import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { useState } from 'react';
import { BrowserEntry, LoadBrowserDirectory } from './file-browser.types';
import { FileTree } from './file-tree';

type FileBrowserProps = {
  queryKey: readonly unknown[];
  rootPath: string;
  rootName: string;
  loadDirectory: LoadBrowserDirectory;
  renderPreview: (entry: BrowserEntry | null) => React.ReactNode;
  enabled?: boolean;
  refreshVersion?: number;
  initialPath?: string | null;
  highlightedPaths?: readonly string[];
  className?: string;
};

export function FileBrowser({
  queryKey,
  rootPath,
  rootName,
  loadDirectory,
  renderPreview,
  enabled,
  refreshVersion,
  initialPath,
  highlightedPaths,
  className,
}: FileBrowserProps) {
  const [selectedEntry, setSelectedEntry] = useState<BrowserEntry | null>(null);
  const [mobileView, setMobileView] = useState<'tree' | 'preview'>('tree');

  const selectEntry = (entry: BrowserEntry) => {
    if (entry.type === 'directory') return;
    setSelectedEntry(entry);
    setMobileView('preview');
  };

  return (
    <div className={cn('flex min-h-0 flex-1 flex-col', className)}>
      <div className="flex gap-1 border-b p-2 md:hidden">
        <Button
          type="button"
          size="sm"
          variant={mobileView === 'tree' ? 'secondary' : 'ghost'}
          onClick={() => setMobileView('tree')}>
          Tree
        </Button>
        <Button
          type="button"
          size="sm"
          variant={mobileView === 'preview' ? 'secondary' : 'ghost'}
          disabled={!selectedEntry}
          onClick={() => setMobileView('preview')}>
          Preview
        </Button>
      </div>
      <div className="grid min-h-0 flex-1 md:grid-cols-[minmax(280px,340px)_minmax(0,1fr)]">
        <div className={cn('min-h-0 overflow-auto border-r', mobileView !== 'tree' && 'hidden md:block')}>
          <FileTree
            queryKey={queryKey}
            rootPath={rootPath}
            rootName={rootName}
            loadDirectory={loadDirectory}
            enabled={enabled}
            refreshVersion={refreshVersion}
            selectedPath={selectedEntry?.path}
            initialPath={initialPath}
            highlightedPaths={highlightedPaths}
            onSelect={selectEntry}
          />
        </div>
        <div className={cn('min-h-0 min-w-0 overflow-hidden', mobileView !== 'preview' && 'hidden md:block')}>
          {renderPreview(selectedEntry)}
        </div>
      </div>
    </div>
  );
}
