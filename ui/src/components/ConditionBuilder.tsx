import { Plus, Trash2, ChevronDown } from 'lucide-react'
import clsx from 'clsx'
import type {
  ConditionNode, ConditionGroup, LeafCondition,
  AllGroup, AnyGroup, NoneGroup, NotGroup, Operator,
} from '../types/ars'
import { isLeaf, isAllGroup, isAnyGroup, isNoneGroup, isNotGroup } from '../types/ars'

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const OPERATORS: { value: Operator; label: string }[] = [
  { value: 'eq',           label: '= equals' },
  { value: 'neq',          label: '≠ not equals' },
  { value: 'gt',           label: '> greater than' },
  { value: 'gte',          label: '≥ greater or equal' },
  { value: 'lt',           label: '< less than' },
  { value: 'lte',          label: '≤ less or equal' },
  { value: 'contains',     label: 'contains' },
  { value: 'starts_with',  label: 'starts with' },
  { value: 'ends_with',    label: 'ends with' },
  { value: 'matches',      label: 'matches (regex)' },
  { value: 'in',           label: 'in list' },
  { value: 'not_in',       label: 'not in list' },
  { value: 'between',      label: 'between' },
  { value: 'outside',      label: 'outside' },
  { value: 'divisible_by', label: 'divisible by' },
  { value: 'is_null',      label: 'is null' },
  { value: 'is_not_null',  label: 'is not null' },
  { value: 'is_empty',     label: 'is empty' },
  { value: 'is_not_empty', label: 'is not empty' },
  { value: 'before',       label: 'before (date)' },
  { value: 'after',        label: 'after (date)' },
  { value: 'within_days',  label: 'within N days' },
  { value: 'is_weekday',   label: 'is weekday' },
  { value: 'is_weekend',   label: 'is weekend' },
  { value: 'contains_any', label: 'contains any' },
  { value: 'contains_all', label: 'contains all' },
  { value: 'length_eq',    label: 'length =' },
  { value: 'length_gt',    label: 'length >' },
  { value: 'length_lt',    label: 'length <' },
  { value: 'is_type',      label: 'is type' },
]

type GroupType = 'all' | 'any' | 'none' | 'not'

function groupType(node: ConditionGroup): GroupType {
  if (isAllGroup(node))  return 'all'
  if (isAnyGroup(node))  return 'any'
  if (isNoneGroup(node)) return 'none'
  return 'not'
}

function groupChildren(node: ConditionGroup): ConditionNode[] {
  if (isAllGroup(node))  return node.all
  if (isAnyGroup(node))  return node.any
  if (isNoneGroup(node)) return node.none
  if (isNotGroup(node))  return [node.not]
  return []
}

function makeGroup(type: GroupType, children: ConditionNode[]): ConditionGroup {
  if (type === 'not') {
    return { not: children[0] ?? newLeaf() }
  }
  const first = children[0] ?? newLeaf()
  return { [type]: children.length ? children : [first] } as ConditionGroup
}

function newLeaf(): LeafCondition {
  return { field: '', op: 'eq', value: '' }
}

function newGroup(): AllGroup {
  return { all: [newLeaf()] }
}

// ---------------------------------------------------------------------------
// Leaf row
// ---------------------------------------------------------------------------

interface LeafProps {
  node: LeafCondition
  onChange: (n: LeafCondition) => void
  onRemove: () => void
  depth: number
}

function noValueNeeded(op: Operator) {
  return ['is_null','is_not_null','is_empty','is_not_empty','is_weekday','is_weekend'].includes(op)
}

function LeafRow({ node, onChange, onRemove }: LeafProps) {
  return (
    <div className="flex items-center gap-2 group">
      <input
        type="text"
        value={node.field}
        onChange={e => onChange({ ...node, field: e.target.value })}
        placeholder="field.path"
        className="flex-1 min-w-0 rounded border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 px-2 py-1.5 text-sm text-gray-900 dark:text-white font-mono focus:outline-none focus:ring-1 focus:ring-brand-500"
      />
      <div className="relative">
        <select
          value={node.op}
          onChange={e => onChange({ ...node, op: e.target.value as Operator })}
          className="appearance-none rounded border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 pl-2 pr-6 py-1.5 text-sm text-gray-700 dark:text-gray-300 focus:outline-none focus:ring-1 focus:ring-brand-500"
        >
          {OPERATORS.map(o => <option key={o.value} value={o.value}>{o.label}</option>)}
        </select>
        <ChevronDown className="absolute right-1.5 top-1/2 -translate-y-1/2 w-3 h-3 pointer-events-none text-gray-400" />
      </div>
      {!noValueNeeded(node.op) && (
        <input
          type="text"
          value={String(node.value ?? '')}
          onChange={e => onChange({ ...node, value: e.target.value })}
          placeholder="value"
          className="flex-1 min-w-0 rounded border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 px-2 py-1.5 text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-1 focus:ring-brand-500"
        />
      )}
      <button
        onClick={onRemove}
        className="opacity-0 group-hover:opacity-100 p-1 text-red-400 hover:text-red-600 transition-opacity"
        title="Remove condition"
      >
        <Trash2 className="w-3.5 h-3.5" />
      </button>
    </div>
  )
}

// ---------------------------------------------------------------------------
// Group node (recursive)
// ---------------------------------------------------------------------------

const GROUP_COLORS: Record<GroupType, string> = {
  all:  'border-blue-300  dark:border-blue-800  bg-blue-50  dark:bg-blue-950/30',
  any:  'border-green-300 dark:border-green-800 bg-green-50 dark:bg-green-950/30',
  none: 'border-orange-300 dark:border-orange-800 bg-orange-50 dark:bg-orange-950/30',
  not:  'border-purple-300 dark:border-purple-800 bg-purple-50 dark:bg-purple-950/30',
}

const GROUP_BADGE: Record<GroupType, string> = {
  all:  'bg-blue-100  text-blue-700  dark:bg-blue-900  dark:text-blue-300',
  any:  'bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300',
  none: 'bg-orange-100 text-orange-700 dark:bg-orange-900 dark:text-orange-300',
  not:  'bg-purple-100 text-purple-700 dark:bg-purple-900 dark:text-purple-300',
}

interface GroupProps {
  node: ConditionGroup
  onChange: (n: ConditionGroup) => void
  onRemove?: () => void
  depth?: number
}

export function ConditionGroupNode({ node, onChange, onRemove, depth = 0 }: GroupProps) {
  const type  = groupType(node)
  const children = groupChildren(node)

  function setChildren(newChildren: ConditionNode[]) {
    onChange(makeGroup(type, newChildren))
  }

  function setType(newType: GroupType) {
    const cur = groupChildren(node)
    // `not` only keeps first child
    onChange(makeGroup(newType, newType === 'not' ? cur.slice(0, 1) : cur))
  }

  function updateChild(i: number, child: ConditionNode) {
    if (isNotGroup(node)) {
      onChange({ not: child })
    } else {
      const next = [...children]
      next[i] = child
      setChildren(next)
    }
  }

  function removeChild(i: number) {
    const next = children.filter((_, idx) => idx !== i)
    if (next.length === 0) next.push(newLeaf())
    setChildren(next)
  }

  function addLeaf() {
    setChildren([...children, newLeaf()])
  }

  function addGroup() {
    setChildren([...children, newGroup()])
  }

  const isNot   = type === 'not'
  const hasChild = children.length > 0

  return (
    <div className={clsx('rounded-lg border p-3 space-y-2', GROUP_COLORS[type])}>
      {/* Group header */}
      <div className="flex items-center gap-2">
        <div className="relative">
          <select
            value={type}
            onChange={e => setType(e.target.value as GroupType)}
            className={clsx('appearance-none rounded px-2 py-1 pr-5 text-xs font-bold uppercase tracking-wide border-0 focus:outline-none focus:ring-1 focus:ring-brand-500', GROUP_BADGE[type])}
          >
            <option value="all">ALL (and)</option>
            <option value="any">ANY (or)</option>
            <option value="none">NONE (nor)</option>
            <option value="not">NOT</option>
          </select>
          <ChevronDown className="absolute right-1 top-1/2 -translate-y-1/2 w-3 h-3 pointer-events-none" />
        </div>
        <span className="text-xs text-gray-500 dark:text-gray-400">
          {isNot ? 'Negates a single condition' : `${children.length} condition${children.length !== 1 ? 's' : ''}`}
        </span>
        {onRemove && (
          <button onClick={onRemove} className="ml-auto p-1 text-red-400 hover:text-red-600">
            <Trash2 className="w-3.5 h-3.5" />
          </button>
        )}
      </div>

      {/* Children */}
      <div className="space-y-2 pl-2">
        {children.map((child, i) => (
          isLeaf(child) ? (
            <LeafRow
              key={i}
              node={child}
              depth={depth + 1}
              onChange={updated => updateChild(i, updated)}
              onRemove={() => removeChild(i)}
            />
          ) : (
            <ConditionGroupNode
              key={i}
              node={child as ConditionGroup}
              depth={depth + 1}
              onChange={updated => updateChild(i, updated)}
              onRemove={() => removeChild(i)}
            />
          )
        ))}
      </div>

      {/* Add buttons — hidden inside `not` when it already has a child */}
      {(!isNot || !hasChild) && (
        <div className="flex gap-2 pt-1">
          <button
            onClick={addLeaf}
            className="flex items-center gap-1 text-xs text-gray-500 dark:text-gray-400 hover:text-brand-600 dark:hover:text-brand-400 transition-colors"
          >
            <Plus className="w-3.5 h-3.5" /> Add condition
          </button>
          {depth < 4 && (
            <button
              onClick={addGroup}
              className="flex items-center gap-1 text-xs text-gray-500 dark:text-gray-400 hover:text-brand-600 dark:hover:text-brand-400 transition-colors"
            >
              <Plus className="w-3.5 h-3.5" /> Add group
            </button>
          )}
        </div>
      )}
    </div>
  )
}

// ---------------------------------------------------------------------------
// Top-level export
// ---------------------------------------------------------------------------

interface ConditionBuilderProps {
  value: ConditionGroup
  onChange: (v: ConditionGroup) => void
}

export default function ConditionBuilder({ value, onChange }: ConditionBuilderProps) {
  return <ConditionGroupNode node={value} onChange={onChange} />
}
