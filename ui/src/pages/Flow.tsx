import { useMemo, useCallback } from 'react'
import { useNavigate } from 'react-router-dom'
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
  type Node,
  type Edge,
  type NodeMouseHandler,
} from '@xyflow/react'
import '@xyflow/react/dist/style.css'
import { useRules } from '../api/hooks'
import type { Rule } from '../types/ars'
import clsx from 'clsx'

// ---------------------------------------------------------------------------
// Build graph from rules
// ---------------------------------------------------------------------------

function buildGraph(rules: Rule[]): { nodes: Node[]; edges: Edge[] } {
  const nodes: Node[] = []
  const edges: Edge[] = []
  const edgeSet = new Set<string>()

  // Simple force-layout: arrange in a grid
  const cols = Math.ceil(Math.sqrt(rules.length))

  rules.forEach((rule, i) => {
    const col = i % cols
    const row = Math.floor(i / cols)

    nodes.push({
      id: rule.id,
      position: { x: col * 240, y: row * 130 },
      data: {
        label: rule.name,
        id: rule.id,
        enabled: rule.enabled,
        priority: rule.priority,
        tags: rule.tags,
      },
      type: 'ruleNode',
    })

    // Build edges from actions
    for (const action of rule.actions) {
      if (action.type === 'call_rule') {
        const edgeId = `${rule.id}->${action.rule_id}-call`
        if (!edgeSet.has(edgeId)) {
          edgeSet.add(edgeId)
          edges.push({
            id: edgeId,
            source: rule.id,
            target: action.rule_id,
            label: 'call_rule',
            type: 'smoothstep',
            style: { stroke: '#6366f1' },
            labelStyle: { fontSize: 10, fill: '#6366f1' },
            animated: false,
          })
        }
      }
      if (action.type === 'trigger') {
        // Find rules that handle this event (convention: rules tagged with the event name)
        const targets = rules.filter(r => r.tags.includes(action.event))
        for (const target of targets) {
          const edgeId = `${rule.id}->${target.id}-trigger-${action.event}`
          if (!edgeSet.has(edgeId)) {
            edgeSet.add(edgeId)
            edges.push({
              id: edgeId,
              source: rule.id,
              target: target.id,
              label: `trigger: ${action.event}`,
              type: 'smoothstep',
              style: { stroke: '#f59e0b', strokeDasharray: '5,5' },
              labelStyle: { fontSize: 10, fill: '#f59e0b' },
              animated: true,
            })
          }
        }
      }
    }
  })

  return { nodes, edges }
}

// ---------------------------------------------------------------------------
// Custom node
// ---------------------------------------------------------------------------

interface RuleNodeData {
  label: string
  id: string
  enabled: boolean
  priority: number
  tags: string[]
  [key: string]: unknown
}

function RuleNode({ data }: { data: RuleNodeData }) {
  return (
    <div className={clsx(
      'px-3 py-2 rounded-lg border shadow-sm min-w-[160px] max-w-[200px] bg-white dark:bg-gray-900 text-xs',
      data.enabled
        ? 'border-blue-200 dark:border-blue-800'
        : 'border-gray-200 dark:border-gray-700 opacity-60',
    )}>
      <div className="flex items-center gap-1.5 mb-0.5">
        <span className={clsx('w-2 h-2 rounded-full shrink-0', data.enabled ? 'bg-green-400' : 'bg-gray-300')} />
        <span className="font-mono text-[10px] text-gray-400 truncate">{data.id}</span>
      </div>
      <p className="font-semibold text-gray-900 dark:text-white truncate text-sm">{data.label}</p>
      <div className="mt-1 flex items-center gap-1 flex-wrap">
        <span className="text-gray-400">p{data.priority}</span>
        {data.tags.slice(0, 2).map((t: string) => (
          <span key={t} className="px-1 py-0.5 rounded bg-gray-100 dark:bg-gray-800 text-gray-500 dark:text-gray-400 text-[10px]">{t}</span>
        ))}
      </div>
    </div>
  )
}

const nodeTypes = { ruleNode: RuleNode }

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function FlowPage() {
  const navigate = useNavigate()
  const { data: rules, isLoading } = useRules()

  const { nodes: initialNodes, edges: initialEdges } = useMemo(
    () => (rules ? buildGraph(rules) : { nodes: [], edges: [] }),
    [rules],
  )

  const [nodes, , onNodesChange] = useNodesState(initialNodes)
  const [edges, , onEdgesChange] = useEdgesState(initialEdges)

  const onNodeClick: NodeMouseHandler = useCallback(
    (_event, node) => {
      navigate(`/rules/${encodeURIComponent(node.id)}`)
    },
    [navigate],
  )

  if (isLoading) {
    return (
      <div className="flex justify-center py-24">
        <div className="w-6 h-6 border-2 border-brand-500 border-t-transparent rounded-full animate-spin" />
      </div>
    )
  }

  if (!rules || rules.length === 0) {
    return (
      <div className="px-6 py-8 text-center text-gray-400 dark:text-gray-500">
        <p className="text-lg font-medium mb-2">No rules to visualise</p>
        <p className="text-sm">Add rules with <code className="font-mono">call_rule</code> or <code className="font-mono">trigger</code> actions to see the dependency graph.</p>
      </div>
    )
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900">
        <h1 className="text-lg font-bold text-gray-900 dark:text-white">Flow</h1>
        <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
          Rule dependency graph.
          <span className="inline-flex items-center gap-1 ml-3">
            <span className="w-6 border-b-2 border-indigo-400 inline-block" />
            <span>call_rule</span>
          </span>
          <span className="inline-flex items-center gap-1 ml-3">
            <span className="w-6 border-b-2 border-dashed border-amber-400 inline-block" />
            <span>trigger</span>
          </span>
          &nbsp;· Click a node to open the rule.
        </p>
      </div>

      {/* Flow canvas */}
      <div className="flex-1">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onNodeClick={onNodeClick}
          nodeTypes={nodeTypes}
          fitView
          fitViewOptions={{ padding: 0.2 }}
          minZoom={0.3}
          maxZoom={2}
        >
          <Background color="#e5e7eb" />
          <Controls />
          <MiniMap nodeColor={n => (n.data as RuleNodeData).enabled ? '#6366f1' : '#d1d5db'} />
        </ReactFlow>
      </div>
    </div>
  )
}
