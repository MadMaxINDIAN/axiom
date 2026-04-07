import { useMemo } from 'react'
import { Link } from 'react-router-dom'
import { useRules } from '../api/hooks'
import type { Rule, ConditionNode, Action } from '../types/ars'
import { isLeaf } from '../types/ars'
import clsx from 'clsx'

// ---------------------------------------------------------------------------
// Helpers: flatten conditions and actions into column-friendly summaries
// ---------------------------------------------------------------------------

function leafFields(node: ConditionNode, acc: string[] = []): string[] {
  if (isLeaf(node)) {
    if (!acc.includes(node.field)) acc.push(node.field)
    return acc
  }
  const children =
    'all'  in node ? node.all  :
    'any'  in node ? node.any  :
    'none' in node ? node.none :
    [node.not]
  children.forEach(c => leafFields(c, acc))
  return acc
}

function summarizeConditions(rule: Rule): string {
  const fields = leafFields(rule.conditions)
  return fields.slice(0, 3).join(', ') + (fields.length > 3 ? ` +${fields.length - 3}` : '')
}

function summarizeAction(action: Action): string {
  switch (action.type) {
    case 'set':       return `set ${action.field} = ${JSON.stringify(action.value)}`
    case 'increment': return `increment ${action.field}`
    case 'append':    return `append → ${action.field}`
    case 'tag':       return `tag: ${action.value}`
    case 'trigger':   return `trigger: ${action.event}`
    case 'call_rule': return `call: ${action.rule_id}`
    case 'return':    return `return ${action.value ?? ''}`
    case 'log':       return `log[${action.level}]`
  }
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function TablesPage() {
  const { data: rules, isLoading, isError } = useRules()

  // Collect all unique condition-field columns across all rules
  const conditionCols = useMemo<string[]>(() => {
    if (!rules) return []
    const all = new Set<string>()
    rules.forEach(r => leafFields(r.conditions).forEach(f => all.add(f)))
    return [...all].slice(0, 8) // cap at 8 columns for readability
  }, [rules])

  if (isLoading) {
    return (
      <div className="flex justify-center py-24">
        <div className="w-6 h-6 border-2 border-brand-500 border-t-transparent rounded-full animate-spin" />
      </div>
    )
  }

  if (isError) {
    return <div className="px-6 py-8 text-red-600 dark:text-red-400">Failed to load rules.</div>
  }

  return (
    <div className="px-6 py-6">
      <div className="mb-6">
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">Decision Table</h1>
        <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
          Spreadsheet view of all rules. Each row is a rule; columns are condition fields and action summaries.
          Click a row to open the rule detail.
        </p>
      </div>

      {rules?.length === 0 ? (
        <div className="text-center py-16 text-gray-400 dark:text-gray-500">
          <p>No rules to display.</p>
          <Link to="/rules/new" className="text-brand-500 text-sm">Create a rule →</Link>
        </div>
      ) : (
        <div className="overflow-x-auto rounded-xl border border-gray-200 dark:border-gray-800">
          <table className="w-full text-xs">
            <thead className="bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 sticky top-0">
              <tr>
                {/* Fixed columns */}
                <th className="px-3 py-2.5 text-left font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider whitespace-nowrap">ID</th>
                <th className="px-3 py-2.5 text-left font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider whitespace-nowrap">Name</th>
                <th className="px-3 py-2.5 text-center font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Pri</th>
                <th className="px-3 py-2.5 text-center font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">On</th>
                {/* Dynamic condition columns */}
                {conditionCols.map(col => (
                  <th key={col} className="px-3 py-2.5 text-left font-medium text-blue-500 dark:text-blue-400 uppercase tracking-wider whitespace-nowrap max-w-[120px] truncate" title={col}>
                    {col.split('.').pop()}
                  </th>
                ))}
                {/* Actions column */}
                <th className="px-3 py-2.5 text-left font-medium text-green-600 dark:text-green-400 uppercase tracking-wider">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100 dark:divide-gray-800 bg-white dark:bg-gray-900">
              {rules?.map(rule => {
                const fields = leafFields(rule.conditions)
                return (
                  <tr
                    key={rule.id}
                    className={clsx(
                      'hover:bg-gray-50 dark:hover:bg-gray-800 cursor-pointer transition-colors',
                      !rule.enabled && 'opacity-50',
                    )}
                    onClick={() => { window.location.href = `/rules/${encodeURIComponent(rule.id)}` }}
                  >
                    <td className="px-3 py-2 font-mono text-brand-600 dark:text-brand-400 whitespace-nowrap">{rule.id}</td>
                    <td className="px-3 py-2 font-medium text-gray-900 dark:text-white whitespace-nowrap max-w-[160px] truncate">{rule.name}</td>
                    <td className="px-3 py-2 text-center text-gray-500 dark:text-gray-400">{rule.priority}</td>
                    <td className="px-3 py-2 text-center">
                      <span className={clsx('inline-block w-2 h-2 rounded-full', rule.enabled ? 'bg-green-500' : 'bg-gray-300 dark:bg-gray-600')} />
                    </td>
                    {conditionCols.map(col => (
                      <td key={col} className="px-3 py-2 text-gray-500 dark:text-gray-400">
                        {fields.includes(col) ? (
                          <span className="inline-flex items-center px-1.5 py-0.5 rounded bg-blue-50 dark:bg-blue-950 text-blue-600 dark:text-blue-400 font-medium">
                            ✓
                          </span>
                        ) : (
                          <span className="text-gray-200 dark:text-gray-700">—</span>
                        )}
                      </td>
                    ))}
                    <td className="px-3 py-2 text-gray-600 dark:text-gray-400 max-w-[220px]">
                      <div className="flex flex-wrap gap-1">
                        {rule.actions.slice(0, 3).map((a, i) => (
                          <span key={i} className="px-1.5 py-0.5 rounded text-[10px] bg-green-50 dark:bg-green-950 text-green-700 dark:text-green-400 whitespace-nowrap">
                            {summarizeAction(a)}
                          </span>
                        ))}
                        {rule.actions.length > 3 && (
                          <span className="text-[10px] text-gray-400">+{rule.actions.length - 3}</span>
                        )}
                      </div>
                    </td>
                  </tr>
                )
              })}
            </tbody>
          </table>
        </div>
      )}

      <p className="mt-3 text-xs text-gray-400 dark:text-gray-500">
        Condition columns show fields detected from all rules (up to 8). ✓ = rule uses this field.
        {conditionCols.length === 8 && ' Some columns may be hidden.'}
      </p>
    </div>
  )
}
