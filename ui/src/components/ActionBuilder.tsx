import { Plus, Trash2, GripVertical } from 'lucide-react'
import type { Action, ActionType, LogLevel } from '../types/ars'
import clsx from 'clsx'

const ACTION_TYPES: { value: ActionType; label: string; description: string }[] = [
  { value: 'set',       label: 'Set',       description: 'Write a value to the output context' },
  { value: 'increment', label: 'Increment', description: 'Add a number to a field' },
  { value: 'append',    label: 'Append',    description: 'Push a value to an array field' },
  { value: 'tag',       label: 'Tag',       description: 'Add a tag to the result' },
  { value: 'trigger',   label: 'Trigger',   description: 'Dispatch an outbound webhook event' },
  { value: 'call_rule', label: 'Call rule', description: 'Evaluate another rule synchronously' },
  { value: 'return',    label: 'Return',    description: 'Stop evaluation and return a value' },
  { value: 'log',       label: 'Log',       description: 'Emit a structured log entry' },
]

function newAction(type: ActionType): Action {
  switch (type) {
    case 'set':       return { type, field: '', value: '' }
    case 'increment': return { type, field: '', value: 1 }
    case 'append':    return { type, field: '', value: '' }
    case 'tag':       return { type, value: '' }
    case 'trigger':   return { type, event: '' }
    case 'call_rule': return { type, rule_id: '' }
    case 'return':    return { type, value: undefined }
    case 'log':       return { type, level: 'info', message: '' }
  }
}

interface ActionRowProps {
  action: Action
  index: number
  onChange: (a: Action) => void
  onRemove: () => void
}

function ActionRow({ action, onChange, onRemove }: ActionRowProps) {
  const labelClass = 'text-xs font-medium text-gray-500 dark:text-gray-400 w-16 shrink-0'
  const inputClass = 'flex-1 min-w-0 rounded border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 px-2 py-1.5 text-sm text-gray-900 dark:text-white font-mono focus:outline-none focus:ring-1 focus:ring-brand-500'

  return (
    <div className="flex items-start gap-2 group bg-white dark:bg-gray-900 rounded-lg border border-gray-200 dark:border-gray-800 p-3">
      <GripVertical className="w-4 h-4 text-gray-300 dark:text-gray-600 mt-1.5 shrink-0 cursor-grab" />

      {/* Type badge */}
      <span className="px-2 py-1 rounded text-xs font-bold bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400 uppercase shrink-0 mt-0.5">
        {action.type.replace('_', '\u00A0')}
      </span>

      {/* Fields */}
      <div className="flex-1 space-y-2">
        {action.type === 'set' && (
          <div className="flex items-center gap-2">
            <span className={labelClass}>field</span>
            <input type="text" value={action.field} onChange={e => onChange({ ...action, field: e.target.value })} placeholder="output.field" className={inputClass} />
            <span className={clsx(labelClass, 'w-8')}>←</span>
            <input type="text" value={String(action.value ?? '')} onChange={e => onChange({ ...action, value: e.target.value })} placeholder="value or {{ expr }}" className={inputClass} />
          </div>
        )}

        {action.type === 'increment' && (
          <div className="flex items-center gap-2">
            <span className={labelClass}>field</span>
            <input type="text" value={action.field} onChange={e => onChange({ ...action, field: e.target.value })} placeholder="counter.field" className={inputClass} />
            <span className={clsx(labelClass, 'w-6')}>by</span>
            <input type="number" value={action.value ?? 1} onChange={e => onChange({ ...action, value: Number(e.target.value) })} className="w-20 rounded border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 px-2 py-1.5 text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-1 focus:ring-brand-500" />
          </div>
        )}

        {action.type === 'append' && (
          <div className="flex items-center gap-2">
            <span className={labelClass}>field</span>
            <input type="text" value={action.field} onChange={e => onChange({ ...action, field: e.target.value })} placeholder="array.field" className={inputClass} />
            <span className={clsx(labelClass, 'w-10')}>push</span>
            <input type="text" value={String(action.value ?? '')} onChange={e => onChange({ ...action, value: e.target.value })} placeholder="value" className={inputClass} />
          </div>
        )}

        {action.type === 'tag' && (
          <div className="flex items-center gap-2">
            <span className={labelClass}>tag</span>
            <input type="text" value={action.value} onChange={e => onChange({ ...action, value: e.target.value })} placeholder="tag-name" className={inputClass} />
          </div>
        )}

        {action.type === 'trigger' && (
          <div className="flex items-center gap-2">
            <span className={labelClass}>event</span>
            <input type="text" value={action.event} onChange={e => onChange({ ...action, event: e.target.value })} placeholder="event.name" className={inputClass} />
          </div>
        )}

        {action.type === 'call_rule' && (
          <div className="flex items-center gap-2">
            <span className={labelClass}>rule id</span>
            <input type="text" value={action.rule_id} onChange={e => onChange({ ...action, rule_id: e.target.value })} placeholder="other-rule-id" className={inputClass} />
          </div>
        )}

        {action.type === 'return' && (
          <div className="flex items-center gap-2">
            <span className={labelClass}>value</span>
            <input type="text" value={String(action.value ?? '')} onChange={e => onChange({ ...action, value: e.target.value })} placeholder="optional return value" className={inputClass} />
          </div>
        )}

        {action.type === 'log' && (
          <div className="flex items-center gap-2">
            <select
              value={action.level}
              onChange={e => onChange({ ...action, level: e.target.value as LogLevel })}
              className="rounded border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 px-2 py-1.5 text-sm text-gray-700 dark:text-gray-300 focus:outline-none focus:ring-1 focus:ring-brand-500"
            >
              <option value="debug">debug</option>
              <option value="info">info</option>
              <option value="warn">warn</option>
            </select>
            <input type="text" value={action.message} onChange={e => onChange({ ...action, message: e.target.value })} placeholder="Log message ({{ field }} supported)" className={inputClass} />
          </div>
        )}
      </div>

      <button onClick={onRemove} className="opacity-0 group-hover:opacity-100 p-1 text-red-400 hover:text-red-600 transition-opacity mt-0.5 shrink-0">
        <Trash2 className="w-3.5 h-3.5" />
      </button>
    </div>
  )
}

// ---------------------------------------------------------------------------

interface Props {
  actions: Action[]
  onChange: (actions: Action[]) => void
}

export default function ActionBuilder({ actions, onChange }: Props) {
  function addAction(type: ActionType) {
    onChange([...actions, newAction(type)])
  }

  function updateAction(i: number, a: Action) {
    const next = [...actions]
    next[i] = a
    onChange(next)
  }

  function removeAction(i: number) {
    onChange(actions.filter((_, idx) => idx !== i))
  }

  return (
    <div className="space-y-2">
      {actions.map((action, i) => (
        <ActionRow
          key={i}
          action={action}
          index={i}
          onChange={a => updateAction(i, a)}
          onRemove={() => removeAction(i)}
        />
      ))}

      {/* Add action menu */}
      <div className="flex flex-wrap gap-2 pt-1">
        {ACTION_TYPES.map(({ value, label }) => (
          <button
            key={value}
            onClick={() => addAction(value)}
            className="flex items-center gap-1 px-2.5 py-1.5 rounded-lg border border-dashed border-gray-300 dark:border-gray-700 text-xs text-gray-500 dark:text-gray-400 hover:border-brand-400 hover:text-brand-600 dark:hover:text-brand-400 transition-colors"
          >
            <Plus className="w-3 h-3" /> {label}
          </button>
        ))}
      </div>
    </div>
  )
}
