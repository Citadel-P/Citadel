export type BrowserEntryType = 'directory' | 'file' | 'symlink' | 'submodule';

export type BrowserEntry = {
  name: string;
  path: string;
  type: BrowserEntryType;
  size?: number | string | null;
  secondaryText?: string | null;
  metadataText?: string | null;
};

export type DirectoryLoadResult = {
  entries: BrowserEntry[];
  isTruncated: boolean;
};

export type LoadBrowserDirectory = (path: string, signal: AbortSignal) => Promise<DirectoryLoadResult>;
