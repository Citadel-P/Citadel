import type { ElkNode } from 'elkjs';

export type LayoutStructure = {
  nodes: { id: string; parentId?: string; expanded: boolean }[];
  edges: { id: string; source: string; target: string }[];
};
export type NodePosition = { id: string; x: number; y: number; width: number; height: number };

export function elkGraph(structure: LayoutStructure): ElkNode {
  const childrenOf = (parentId?: string): ElkNode[] =>
    structure.nodes
      .filter((n) => n.parentId === parentId)
      .map((node) => ({
        id: node.id,
        ...(node.expanded
          ? {
              children: childrenOf(node.id),
              layoutOptions: { 'elk.padding': '[top=56,left=20,bottom=20,right=20]', 'elk.direction': 'DOWN' },
            }
          : { width: 244, height: 92 }),
      }));
  return {
    id: 'root',
    layoutOptions: {
      'elk.algorithm': 'layered',
      'elk.direction': 'DOWN',
      'elk.hierarchyHandling': 'INCLUDE_CHILDREN',
      'elk.spacing.nodeNode': '24',
      'elk.layered.spacing.nodeNodeBetweenLayers': '64',
      'elk.randomSeed': '1',
    },
    children: childrenOf(),
    edges: structure.edges.map(({ id, source, target }) => ({ id, sources: [source], targets: [target] })),
  };
}

export function layoutPositions(root: ElkNode): NodePosition[] {
  return (root.children ?? []).flatMap((node) => [
    { id: node.id, x: node.x ?? 0, y: node.y ?? 0, width: node.width ?? 244, height: node.height ?? 92 },
    ...layoutPositions(node),
  ]);
}
