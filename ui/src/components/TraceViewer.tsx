import { useState } from 'react'
import { ChevronRight, ChevronDown, CheckCircle2, XCircle, Clock } from 'lucide-react'
import type { EvaluationResponse, RuleTrace } from '../types/ars'
import clsx from 'clsx'

// ---------------------------------------------------------------------------
// Rule trace row
// ---------------------------------------------------------------------------

function RuleTraceRow({ rule }: { rule: RuleTrace }) {
  const [open, setOpen] = useState(rule.matched)

  return (
    <div className={clsx(
      'rounded-lg border overflow-hidden text-sm',
      rule.matched
        ? 'border-green-200 dark:border-green-800'
        : 'border-gray-200 dark:border-gray-800',
    )}>
      {/* Header */}
      <button
        onClick={() => setOpen(v => !v)}
        className={clsx(
          'w-full flex items-center gap-2 px-3 py-2 text-left',
          rule.matched
            ? 'bg-green-50 dark:bg-green-950/40'
            : 'bg-gray-50 dark:bg-gray-800/40',
        )}
      >
        {open ? <ChevronDown className="w-3.5 h-3.5 text-gray-400 shrink-0" /> : <ChevronRight className="w-3.5 h-3.5 text-gray-400 shrink-0" />}
        {rule.matched
          ? <CheckCircle2 className="w-3.5 h-3.5 text-green-500 shrink-0" />
          : <XCircle className="w-3.5 h-3.5 text-gray-400 shrink-0" />}
        <span className={clsx('font-mono font-medium', rule.matched ? 'text-green-800 dark:text-green-300' : 'text-gray-600 dark:text-gray-400')}>
          {rule.rule_id}
        </span>
        {rule.short_circuited && (
          <span className="ml-1 text-xs text-gray-400">(short-circuited)</span>
        )}
        <span className="ml-auto text-xs text-gray-400 flex items-center gap-1">
          <Clock className="w-3 h-3" />{rule.duration_us} µs
        </span>
      </button>

      {/* Conditions */}
      {open && (
        <div className="px-3 py-2 space-y-1 bg-white dark:bg-gray-900">
          {rule.conditions.map((c, i) => (
            <div key={i} className="flex items-center gap-2 text-xs">
              {c.passed
                ? <CheckCircle2 className="w-3 h-3 text-green-500 shrink-0" />
                : <XCircle className="w-3 h-3 text-red-400 shrink-0" />}
              {c.field && (
                <code className="font-mono text-gray-700 dark:text-gray-300">{c.field}</code>
              )}
              {c.operator && (
                <span className="text-gray-400">{c.operator}</span>
              )}
              {c.value !== undefined && (
                <code className="font-mono text-brand-600 dark:text-brand-400">
                  {JSON.stringify(c.value)}
                </code>
              )}
              {c.actual_value !== undefined && (
                <span className="text-gray-400 ml-auto">
                  actual: <code className="font-mono">{JSON.stringify(c.actual_value)}</code>
                </span>
              )}
            </div>
          ))}

          {rule.actions_executed.length > 0 && (
            <div className="mt-2 pt-2 border-t border-gray-100 dark:border-gray-800">
              <span className="text-xs font-medium text-gray-500 dark:text-gray-400">Actions executed:</span>
              <ul className="mt-1 space-y-0.5">
                {rule.actions_executed.map((a, i) => (
                  <li key={i} className="text-xs font-mono text-gray-600 dark:text-gray-400 pl-2">· {a}</li>
                ))}
              </ul>
            </div>
          )}
        </div>
      )}
    </div>
  )
}

// ---------------------------------------------------------------------------
// Top-level viewer
// ---------------------------------------------------------------------------

interface Props {
  result: EvaluationResponse
}

export default function TraceViewer({ result }: Props) {
  return (
    <div className="space-y-4">
      {/* Summary */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
        {[
          { label: 'Matched', value: result.matched ? 'Yes' : 'No', color: result.matched ? 'text-green-600 dark:text-green-400' : 'text-gray-500' },
          { label: 'Rules evaluated', value: result.trace.rules_evaluated },
          { label: 'Rules matched', value: result.trace.rules_matched },
          { label: 'Duration', value: `${result.duration_us} µs` },
        ].map(({ label, value, color }) => (
          <div key={label} className="rounded-lg bg-gray-50 dark:bg-gray-800 px-3 py-2">
            <p className="text-xs text-gray-500 dark:text-gray-400">{label}</p>
            <p className={clsx('text-sm font-semibold mt-0.5', color ?? 'text-gray-900 dark:text-white')}>{String(value)}</p>
          </div>
        ))}
      </div>

      {/* Tags */}
      {result.tags.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {result.tags.map(tag => (
            <span key={tag} className="px-2 py-1 rounded-full text-xs bg-brand-100 dark:bg-brand-950 text-brand-700 dark:text-brand-300 font-medium">
              {tag}
            </span>
          ))}
        </div>
      )}

      {/* Output context */}
      {Object.keys(result.output_context).length > 0 && (
        <details className="rounded-lg border border-gray-200 dark:border-gray-800 overflow-hidden">
          <summary className="px-3 py-2 bg-gray-50 dark:bg-gray-800 text-xs font-medium text-gray-600 dark:text-gray-400 cursor-pointer">
            Output context
          </summary>
          <pre className="px-3 py-2 text-xs font-mono text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-900 overflow-auto max-h-40">
            {JSON.stringify(result.output_context, null, 2)}
          </pre>
        </details>
      )}

      {/* Per-rule traces */}
      <div className="space-y-2">
        {result.trace.rules.map(rule => (
          <RuleTraceRow key={rule.rule_id} rule={rule} />
        ))}
      </div>

      {result.trace.timed_out && (
        <p className="text-xs text-orange-600 dark:text-orange-400 font-medium">
          ⚠ Evaluation timed out — trace may be incomplete.
        </p>
      )}
    </div>
  )
}
