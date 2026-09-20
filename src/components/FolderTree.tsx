import type { FolderNode } from "../lib/folderTree";

type Props = {
  tree: FolderNode;
  selectedKey: string;
  expanded: Set<string>;
  onSelect: (node: FolderNode) => void;
  onToggle: (key: string) => void;
};

export function FolderTree({ tree, selectedKey, expanded, onSelect, onToggle }: Props) {
  return (
    <nav
      className="flex h-full min-h-0 min-w-0 flex-col border-r border-md-border bg-md-surface/40"
      aria-label="Library folders"
    >
      <p className="shrink-0 px-3 py-2 text-[11px] uppercase tracking-wide text-md-muted">
        Folders
      </p>
      <div role="tree" className="min-h-0 flex-1 overflow-auto px-1 pb-2">
        <FolderRow
          node={tree}
          depth={0}
          selectedKey={selectedKey}
          expanded={expanded}
          onSelect={onSelect}
          onToggle={onToggle}
        />
      </div>
    </nav>
  );
}

function FolderRow({
  node,
  depth,
  selectedKey,
  expanded,
  onSelect,
  onToggle,
}: {
  node: FolderNode;
  depth: number;
  selectedKey: string;
  expanded: Set<string>;
  onSelect: (node: FolderNode) => void;
  onToggle: (key: string) => void;
}) {
  const hasChildren = node.children.length > 0;
  const open = expanded.has(node.key);
  const selected = node.key === selectedKey;

  return (
    <div>
      <div
        role="treeitem"
        aria-selected={selected}
        aria-expanded={hasChildren ? open : undefined}
        title={node.title ?? node.label}
        onClick={() => onSelect(node)}
        className={`flex cursor-pointer items-center gap-1 rounded-md py-0.5 pr-2 text-sm ${
          selected
            ? "bg-md-violet/35 text-md-cyan"
            : "text-md-text hover:bg-md-border/60"
        }`}
        style={{ paddingLeft: 6 + depth * 12 }}
      >
        <span
          className="flex h-4 w-4 shrink-0 items-center justify-center text-[10px] text-md-muted"
          onClick={(event) => {
            event.stopPropagation();
            if (hasChildren) {
              onToggle(node.key);
            }
          }}
        >
          {hasChildren ? (open ? "▾" : "▸") : ""}
        </span>
        <span className="min-w-0 flex-1 truncate">{node.label}</span>
        <span className="shrink-0 text-[11px] tabular-nums text-md-muted">{node.count}</span>
      </div>
      {hasChildren && open
        ? node.children.map((child) => (
            <FolderRow
              key={child.key}
              node={child}
              depth={depth + 1}
              selectedKey={selectedKey}
              expanded={expanded}
              onSelect={onSelect}
              onToggle={onToggle}
            />
          ))
        : null}
    </div>
  );
}
