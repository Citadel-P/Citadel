import { useEffect, useMemo, useRef, useState } from 'react';
import {
  Background,
  Controls,
  Handle,
  Position,
  ReactFlow,
  type Node,
  type NodeProps,
  type ReactFlowInstance,
} from '@xyflow/react';
import { ChevronDown, ChevronRight } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { highlightedNodes, layoutStructure, type MapNode, type NetworkGraph } from './model';
import { elkGraph, layoutPositions, type NodePosition } from './layout';
import { MapNodeIcon } from './node-icon';
import ELK from 'elkjs/lib/elk-api';
import type { ElkNode } from 'elkjs';
import elkWorkerUrl from 'elkjs/lib/elk-worker.min.js?url';
import '@xyflow/react/dist/style.css';
import './map.css';

type NodeData = MapNode & { related: boolean; onToggle: (id: string) => void; onSelect: (id: string) => void };
type FlowNode = Node<NodeData, 'resource'>;

function ResourceNode({ data, selected }: NodeProps<FlowNode>) {
  return (
    <div
      className={cn(
        'h-full w-full rounded-xl border border-border/80 bg-card text-card-foreground shadow-sm transition-[border-color,box-shadow]',
        selected && 'border-primary/50 ring-2 ring-primary/10',
        !selected && data.related && 'border-primary/30',
        data.expanded && 'border-dashed bg-muted/20 shadow-none',
        data.kind === 'reference' && 'border-dashed',
      )}>
      {data.kind !== 'network' && !data.expanded && (
        <Handle type="target" position={Position.Top} isConnectable={false} />
      )}
      <div className={cn('flex items-start gap-2.5 p-3', data.expanded && 'bg-card')}>
        <div className="flex size-8 shrink-0 items-center justify-center">
          <MapNodeIcon node={data} className="size-4" />
        </div>
        <button
          type="button"
          className="nodrag min-w-0 flex-1 text-left outline-none focus-visible:ring-2 focus-visible:ring-ring"
          onClick={() => data.onSelect(data.id)}
          aria-label={`Show details for ${data.label}`}>
          <span className="block truncate text-[13px] font-medium leading-5" title={data.label}>
            {data.label}
          </span>
          {!data.expanded && (
            <>
              <span className="mt-1 block truncate text-xs text-muted-foreground" title={data.subtitle}>
                {data.subtitle}
              </span>
              {data.address && (
                <span className="mt-1 block truncate font-mono text-[11px] text-muted-foreground" title={data.address}>
                  {data.address}
                </span>
              )}
            </>
          )}
        </button>
        {data.kind === 'stack' && (
          <Button
            variant="ghost"
            size="icon"
            className="nodrag -mt-1 size-7"
            aria-label={`${data.expanded ? 'Collapse' : 'Expand'} ${data.label}`}
            aria-expanded={!!data.expanded}
            onClick={(event) => {
              event.stopPropagation();
              data.onToggle(data.id);
            }}>
            {data.expanded ? <ChevronDown className="size-4" /> : <ChevronRight className="size-4" />}
          </Button>
        )}
      </div>
      {data.kind === 'network' && <Handle type="source" position={Position.Bottom} isConnectable={false} />}
    </div>
  );
}
const nodeTypes = { resource: ResourceNode };

export default function NetworkCanvas({
  graph,
  selected,
  onSelect,
  onToggle,
  reset,
}: {
  graph: NetworkGraph;
  selected?: string;
  onSelect: (id: string) => void;
  onToggle: (id: string) => void;
  reset: number;
}) {
  const structure = layoutStructure(graph);
  const host = useRef<HTMLDivElement>(null);
  const flow = useRef<ReactFlowInstance<FlowNode> | null>(null);
  const lastFit = useRef<string | undefined>(undefined);
  const [layout, setLayout] = useState<{ structure: string; reset: number; positions: NodePosition[] }>();
  const [error, setError] = useState<{ structure: string; reset: number }>();
  useEffect(() => {
    let worker: Worker | undefined;
    let active = true;
    // The promise also captures failures constructing the browser worker.
    const result = new Promise<ElkNode>((resolve, reject) => {
      worker = new Worker(elkWorkerUrl);
      const elk = new ELK({ workerFactory: () => worker! });
      worker.onerror = () => reject(new Error('Layout worker failed'));
      void elk.layout(elkGraph(JSON.parse(structure))).then(resolve, reject);
    });
    void result
      .then((value) => {
        if (active) setLayout({ structure, reset, positions: layoutPositions(value) });
      })
      .catch(() => {
        if (active) setError({ structure, reset });
      });
    // Cancels obsolete layouts on a topology change, reset, focus switch or unmount.
    return () => {
      active = false;
      worker?.terminate();
    };
  }, [structure, reset]);
  useEffect(() => {
    const key = JSON.stringify([structure, reset]);
    if (!layout || layout.structure !== structure || layout.reset !== reset || lastFit.current === key) return;
    const frame = requestAnimationFrame(() => {
      void flow.current?.fitView({ padding: 0.22, maxZoom: 1 });
      lastFit.current = key;
    });
    return () => cancelAnimationFrame(frame);
  }, [layout, structure, reset]);
  useEffect(() => {
    const element = host.current;
    if (!element) return;
    let frame = 0;
    const observer = new ResizeObserver(() => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        void flow.current?.fitView({ padding: 0.22, maxZoom: 1 });
      });
    });
    observer.observe(element);
    return () => {
      observer.disconnect();
      cancelAnimationFrame(frame);
    };
  }, []);
  const related = useMemo(() => highlightedNodes(graph, selected), [graph, selected]);
  const nodes = useMemo<FlowNode[]>(() => {
    const positions = new Map(layout?.positions.map((position) => [position.id, position]));
    return graph.nodes.flatMap((node) => {
      const position = positions.get(node.id);
      if (!position) return [];
      return [
        {
          id: node.id,
          parentId: node.parentId,
          type: 'resource',
          zIndex: 1,
          position: { x: position.x, y: position.y },
          width: position.width,
          height: position.height,
          style: { width: position.width, height: position.height },
          selected: selected === node.id,
          data: { ...node, onToggle, onSelect, related: related.has(node.id) },
          ariaLabel: `${node.label}, ${node.subtitle}`,
        },
      ];
    });
  }, [graph.nodes, layout, selected, onToggle, onSelect, related]);
  const edges = useMemo(
    () =>
      graph.edges.map(({ saved, service, ...edge }) => ({
        ...edge,
        type: 'default',
        zIndex: 0,
        label: saved ? 'Configured use' : service ? 'Service network' : undefined,
        style: {
          stroke:
            related.has(edge.source) && related.has(edge.target)
              ? 'color-mix(in srgb, var(--primary) 55%, transparent)'
              : 'color-mix(in srgb, var(--muted-foreground) 35%, transparent)',
          strokeWidth: 1.5,
          ...(saved ? { strokeDasharray: '5 5' } : {}),
        },
      })),
    [graph.edges, related],
  );
  if (error?.structure === structure && error.reset === reset)
    return (
      <p role="alert" className="p-6 text-sm text-muted-foreground">
        The map layout could not be created. Use the details panel to view attachments.
      </p>
    );
  // Hide outdated relationships until the current bounded layout is ready.
  const pending = layout?.structure !== structure || layout?.reset !== reset;
  return (
    <div ref={host} className="network-map relative h-full">
      {pending && (
        <p role="status" className="absolute inset-x-0 top-4 z-10 text-center text-sm text-muted-foreground">
          Arranging map…
        </p>
      )}
      {layout && (
        <ReactFlow<FlowNode>
          onInit={(instance) => {
            flow.current = instance;
          }}
          nodes={pending ? [] : nodes}
          edges={pending ? [] : edges}
          nodeTypes={nodeTypes}
          zIndexMode="manual"
          onNodeClick={(_, node) => onSelect(node.id)}
          nodesDraggable={false}
          nodesConnectable={false}
          edgesReconnectable={false}
          deleteKeyCode={null}
          proOptions={{ hideAttribution: true }}
          minZoom={0.15}
          maxZoom={1.5}
          fitView
          fitViewOptions={{ padding: 0.2, maxZoom: 1 }}
          aria-label="Network attachments map">
          <Background color="var(--border)" gap={24} size={1} />
          <Controls
            showInteractive={false}
            orientation="horizontal"
            position="bottom-right"
            fitViewOptions={{ padding: 0.22, maxZoom: 1 }}
          />
        </ReactFlow>
      )}
    </div>
  );
}
