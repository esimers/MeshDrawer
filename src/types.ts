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
  sha256: string | null;
  ignoredDupe: boolean;
};

export type SlicerApp = {
  id: string;
  name: string;
  path: string | null;
  available: boolean;
};

export type PlateRow = {
  plateIndex: number;
  name: string | null;
  thumbPath: string | null;
  objectIds: number[];
};

export type ThumbReady = {
  id: number;
  thumbPath: string | null;
  thumbStatus: string;
  plateCount: number;
};

export type HashProgress = {
  running: boolean;
  hashed: number;
  remaining: number;
  currentPath: string;
};

export type DuplicateGroup = {
  sha256: string;
  size: number;
  files: FileRow[];
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
