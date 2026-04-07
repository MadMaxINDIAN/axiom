import { useState, useEffect, useRef } from 'react'
import { Play, RefreshCw } from 'lucide-react'
import { useEvaluate } from '../api/hooks'
import TraceViewer from './TraceViewer'
import type { Strategy } from '../types/ars'

const DEBOUNCE_MS = 300

interface Props {
  ruleId?: string
  rulesetName?: string
  autoRun?: boolean
}

export default function TestPanel({ ruleId, rulesetName, autoRun = false }: Props) {
  const [contextText, setContextText] = useState('{\n  \n}')
  const [strategy, setStrategy]       = useState<Strategy>('all_match')
  const [parseError, setParseError]   = useState<string | null>(null)
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  const evaluate = useEvaluate()

  function run(contextStr: string) {
    let ctx: Record<string, unknown>
    try {
      ctx = JSON.parse(contextStr)
      setParseError(null)
    } catch (e: unknown) {
      setParseError(e instanceof Error ? e.message : 'Invalid JSON')
      return
    }

    evaluate.mutate({
      context: ctx,
      dry_run: true,
      strategy,
      ...(ruleId     ? { rule_id: ruleId }       : {}),
      ...(rulesetName ? { ruleset: rulesetName }  : {}),
    })
  }

  // Auto-run on context change with debounce
  useEffect(() => {
    if (!autoRun) return
    if (debounceRef.current) clearTimeout(debounceRef.current)
    debounceRef.current = setTimeout(() => run(contextText), DEBOUNCE_MS)
    return () => { if (debounceRef.current) clearTimeout(debounceRef.current) }
  }, [contextText, strategy, autoRun]) // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <div className="space-y-4">
      {/* Context input */}
      <div>
        <div className="flex items-center justify-between mb-1.5">
          <label className="text-xs font-medium text-gray-700 dark:text-gray-300">Context JSON</label>
          <div className="flex items-center gap-2">
            <select
              value={strategy}
              onChange={e => setStrategy(e.target.value as Strategy)}
              className="rounded border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 px-2 py-1 text-xs text-gray-700 dark:text-gray-300 focus:outline-none focus:ring-1 focus:ring-brand-500"
            >
              <option value="first_match">First match</option>
              <option value="all_match">All match</option>
              <option value="scored">Scored</option>
            </select>
            {!autoRun && (
              <button
                onClick={() => run(contextText)}
                disabled={evaluate.isPending}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-brand-600 text-white text-xs font-medium hover:bg-brand-700 disabled:opacity-50 transition-colors"
              >
                {evaluate.isPending
                  ? <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                  : <Play className="w-3.5 h-3.5" />}
                Run
              </button>
            )}
          </div>
        </div>
        <textarea
          value={contextText}
          onChange={e => setContextText(e.target.value)}
          rows={8}
          spellCheck={false}
          className="w-full rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 px-3 py-2 text-sm font-mono text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-brand-500 resize-y"
        />
        {parseError && (
          <p className="mt-1 text-xs text-red-500">{parseError}</p>
        )}
      </div>

      {/* Auto-run indicator */}
      {autoRun && (
        <p className="text-xs text-gray-400 dark:text-gray-500">
          {evaluate.isPending ? '⏳ Evaluating…' : 'Evaluates automatically as you type (dry-run mode)'}
        </p>
      )}

      {/* Error */}
      {evaluate.isError && (
        <div className="rounded-lg border border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-950 px-4 py-3 text-sm text-red-700 dark:text-red-300">
          {evaluate.error instanceof Error ? evaluate.error.message : 'Evaluation failed'}
        </div>
      )}

      {/* Result */}
      {evaluate.data && <TraceViewer result={evaluate.data} />}
    </div>
  )
}
