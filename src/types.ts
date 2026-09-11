export type LibraryRoot = {
  id: number;
  path: string;
  addedAt: number;
};

export type AppInfo = {
  dbPath: string;
  thumbsDir: string;
  dataDir: string;
};

export type FileRow = {
  id: number;
  rootId: number;
  relPath: string;
  absPath: string;
  ext: string;
  size: number;
  mtimeMs: number;
  thumbStatus: string;
  thumbPath: string | null;
  plateCount: number;
  triangleCount: number | null;
  lastError: string | null;
};

export type SlicerApp = {
  id: string;
  name: string;
  path: string | null;
  available: boolean;
};

export type ThumbReady = {
  id: number;
  thumbPath: string | null;
  thumbStatus: string;
  plateCount: number;
};

export type ScanProgress = {
  running: boolean;
  paused: boolean;
  seen: number;
  indexed: number;
  errors: number;
  currentPath: string;
  rootPath: string;
};
