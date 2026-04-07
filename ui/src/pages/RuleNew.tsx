import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { Save, Code2, Eye } from 'lucide-react'
import { stringify as yamlStringify } from 'yaml'
import ConditionBuilder from '../components/ConditionBuilder'
import ActionBuilder from '../components/ActionBuilder'
import { useCreateRule } from '../api/hooks'
import type { Rule, ConditionGroup, Action } from '../types/ars'

function defaultConditions(): ConditionGroup {
  return { all: [{ field: '', op: 'eq', value: '' }] }
}

export default function RuleNewPage() {
  const navigate = useNavigate()
  const createRule = useCreateRule()

  const [id, setId]               = useState('')
  const [name, setName]           = useState('')
  const [description, setDesc]    = useState('')
  const [priority, setPriority]   = useState(10)
  const [tags, setTags]           = useState('')
  const [conditions, setCond]     = useState<ConditionGroup>(defaultConditions)
  const [actions, setActions]     = useState<Action[]>([{ type: 'tag', value: '' }])
  const [showSource, setSource]   = useState(false)
  const [error, setError]         = useState<string | null>(null)

  const rule: Omit<Rule, 'version'> = {
    ars_version: 1,
    id: id.trim(),
    name: name.trim(),
    description: description.trim() || undefined,
    priority,
    enabled: true,
    tags: tags.split(',').map(t => t.trim()).filter(Boolean),
    conditions,
    actions,
  }

  async function handleSave() {
    setError(null)
    try {
      await createRule.mutateAsync(rule)
      navigate(`/rules/${encodeURIComponent(rule.id)}`)
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e))
    }
  }

  return (
    <div className="max-w-3xl mx-auto px-6 py-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">New rule</h1>
        <div className="flex gap-2">
          <button
            onClick={() => setSource(v => !v)}
            className="flex items-center gap-2 px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-700 text-sm text-gray-600 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
          >
            {showSource ? <Eye className="w-4 h-4" /> : <Code2 className="w-4 h-4" />}
            {showSource ? 'Visual editor' : 'View source'}
          </button>
          <button
            onClick={handleSave}
            disabled={createRule.isPending || !id || !name}
            className="flex items-center gap-2 px-4 py-2 rounded-lg bg-brand-600 text-white text-sm font-medium hover:bg-brand-700 disabled:opacity-50 transition-colors"
          >
            <Save className="w-4 h-4" />
            {createRule.isPending ? 'Saving…' : 'Save rule'}
          </button>
        </div>
      </div>

      {error && (
        <div className="rounded-lg bg-red-50 dark:bg-red-950 border border-red-200 dark:border-red-800 px-4 py-3 text-sm text-red-700 dark:text-red-300">
          {error}
        </div>
      )}

      {showSource ? (
        <pre className="rounded-xl border border-gray-200 dark:border-gray-800 bg-gray-50 dark:bg-gray-900 p-4 text-xs font-mono text-gray-800 dark:text-gray-200 overflow-auto max-h-[70vh] whitespace-pre-wrap">
          {yamlStringify({ ...rule, version: 1 })}
        </pre>
      ) : (
        <div className="space-y-6">
          {/* Identity */}
          <section className="bg-white dark:bg-gray-900 rounded-xl border border-gray-200 dark:border-gray-800 p-5 space-y-4">
            <h2 className="font-semibold text-gray-900 dark:text-white text-sm uppercase tracking-wide text-gray-500 dark:text-gray-400">Identity</h2>
            <div className="grid grid-cols-2 gap-4">
              <label className="block">
                <span className="text-xs font-medium text-gray-700 dark:text-gray-300">ID <span className="text-red-500">*</span></span>
                <input type="text" value={id} onChange={e => setId(e.target.value)} placeholder="my-rule-id" className="mt-1 block w-full rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 px-3 py-2 text-sm font-mono text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-brand-500" />
              </label>
              <label className="block">
                <span className="text-xs font-medium text-gray-700 dark:text-gray-300">Name <span className="text-red-500">*</span></span>
                <input type="text" value={name} onChange={e => setName(e.target.value)} placeholder="Human-readable name" className="mt-1 block w-full rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 px-3 py-2 text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-brand-500" />
              </label>
            </div>
            <label className="block">
              <span className="text-xs font-medium text-gray-700 dark:text-gray-300">Description</span>
              <textarea value={description} onChange={e => setDesc(e.target.value)} rows={2} placeholder="What does this rule do?" className="mt-1 block w-full rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 px-3 py-2 text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-brand-500 resize-none" />
            </label>
            <div className="grid grid-cols-2 gap-4">
              <label className="block">
                <span className="text-xs font-medium text-gray-700 dark:text-gray-300">Priority</span>
                <input type="number" value={priority} onChange={e => setPriority(Number(e.target.value))} className="mt-1 block w-full rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 px-3 py-2 text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-brand-500" />
              </label>
              <label className="block">
                <span className="text-xs font-medium text-gray-700 dark:text-gray-300">Tags (comma separated)</span>
                <input type="text" value={tags} onChange={e => setTags(e.target.value)} placeholder="finance, loan" className="mt-1 block w-full rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 px-3 py-2 text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-brand-500" />
              </label>
            </div>
          </section>

          {/* Conditions */}
          <section className="bg-white dark:bg-gray-900 rounded-xl border border-gray-200 dark:border-gray-800 p-5 space-y-3">
            <h2 className="font-semibold text-gray-900 dark:text-white text-sm">Conditions</h2>
            <ConditionBuilder value={conditions} onChange={setCond} />
          </section>

          {/* Actions */}
          <section className="bg-white dark:bg-gray-900 rounded-xl border border-gray-200 dark:border-gray-800 p-5 space-y-3">
            <h2 className="font-semibold text-gray-900 dark:text-white text-sm">Actions</h2>
            <ActionBuilder actions={actions} onChange={setActions} />
          </section>
        </div>
      )}
    </div>
  )
}
