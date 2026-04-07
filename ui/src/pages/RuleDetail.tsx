import { useState } from 'react'
import { useParams, Link, useNavigate } from 'react-router-dom'
import { ArrowLeft, Edit2, Trash2, ToggleLeft, ToggleRight, History, FlaskConical, Code2 } from 'lucide-react'
import { stringify as yamlStringify } from 'yaml'
import { useRule, useRuleVersions, usePatchRule, useDeleteRule } from '../api/hooks'
import { useSettingsStore } from '../store/settings'
import TestPanel from '../components/TestPanel'
import clsx from 'clsx'

type Tab = 'overview' | 'test' | 'history' | 'source'

export default function RuleDetailPage() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const { role } = useSettingsStore()
  const [tab, setTab] = useState<Tab>('overview')

  const { data: rule, isLoading, isError } = useRule(id!)
  const { data: versions } = useRuleVersions(id!)
  const patchRule  = usePatchRule(id!)
  const deleteRule = useDeleteRule()

  const canEdit = role === 'admin' || role === 'editor'

  async function toggleEnabled() {
    if (!rule) return
    await patchRule.mutateAsync({ enabled: !rule.enabled })
  }

  async function handleDelete() {
    if (!confirm('Disable and remove this rule?')) return
    await deleteRule.mutateAsync(id!)
    navigate('/rules')
  }

  const tabs: { id: Tab; label: string; icon: React.ElementType }[] = [
    { id: 'overview', label: 'Overview', icon: Edit2 },
    { id: 'test',     label: 'Test',     icon: FlaskConical },
    { id: 'history',  label: 'History',  icon: History },
    { id: 'source',   label: 'Source',   icon: Code2 },
  ]

  if (isLoading) {
    return (
      <div className="flex justify-center py-24">
        <div className="w-6 h-6 border-2 border-brand-500 border-t-transparent rounded-full animate-spin" />
      </div>
    )
  }

  if (isError || !rule) {
    return (
      <div className="px-6 py-8">
        <p className="text-red-600 dark:text-red-400">Rule not found.</p>
        <Link to="/rules" className="text-brand-500 underline text-sm">← Back to rules</Link>
      </div>
    )
  }

  return (
    <div className="px-6 py-6 max-w-3xl mx-auto">
      {/* Back */}
      <Link to="/rules" className="flex items-center gap-1.5 text-sm text-gray-500 hover:text-gray-800 dark:hover:text-gray-200 mb-4 transition-colors">
        <ArrowLeft className="w-4 h-4" /> Rules
      </Link>

      {/* Header */}
      <div className="flex items-start justify-between gap-4 mb-6">
        <div>
          <div className="flex items-center gap-2 mb-1">
            <span className="text-xs font-mono text-gray-400">{rule.id}</span>
            <span className={clsx(
              'px-2 py-0.5 rounded-full text-xs font-medium',
              rule.enabled
                ? 'bg-green-100 text-green-700 dark:bg-green-950 dark:text-green-400'
                : 'bg-gray-100 text-gray-500 dark:bg-gray-800 dark:text-gray-400',
            )}>
              {rule.enabled ? 'Enabled' : 'Disabled'}
            </span>
          </div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-white">{rule.name}</h1>
          {rule.description && <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">{rule.description}</p>}
        </div>

        {canEdit && (
          <div className="flex gap-2 shrink-0">
            <button
              onClick={toggleEnabled}
              disabled={patchRule.isPending}
              className="flex items-center gap-1.5 px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-700 text-sm text-gray-600 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
              title={rule.enabled ? 'Disable rule' : 'Enable rule'}
            >
              {rule.enabled
                ? <ToggleRight className="w-4 h-4 text-green-500" />
                : <ToggleLeft className="w-4 h-4 text-gray-400" />}
              {rule.enabled ? 'Disable' : 'Enable'}
            </button>
            {role === 'admin' && (
              <button
                onClick={handleDelete}
                className="flex items-center gap-1.5 px-3 py-2 rounded-lg border border-red-300 dark:border-red-800 text-sm text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-950 transition-colors"
              >
                <Trash2 className="w-4 h-4" /> Delete
              </button>
            )}
          </div>
        )}
      </div>

      {/* Tabs */}
      <div className="flex gap-1 border-b border-gray-200 dark:border-gray-800 mb-5">
        {tabs.map(({ id: tid, label, icon: Icon }) => (
          <button
            key={tid}
            onClick={() => setTab(tid)}
            className={clsx(
              'flex items-center gap-1.5 px-4 py-2.5 text-sm font-medium border-b-2 -mb-px transition-colors',
              tab === tid
                ? 'border-brand-500 text-brand-600 dark:text-brand-400'
                : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200',
            )}
          >
            <Icon className="w-3.5 h-3.5" />{label}
          </button>
        ))}
      </div>

      {/* Tab content */}
      {tab === 'overview' && (
        <div className="space-y-4">
          {/* Metadata */}
          <div className="grid grid-cols-3 gap-3">
            {[
              { label: 'Version', value: `v${rule.version}` },
              { label: 'Priority', value: rule.priority },
              { label: 'Actions', value: rule.actions.length },
            ].map(({ label, value }) => (
              <div key={label} className="rounded-lg bg-gray-50 dark:bg-gray-800 px-3 py-2">
                <p className="text-xs text-gray-500 dark:text-gray-400">{label}</p>
                <p className="text-sm font-semibold text-gray-900 dark:text-white mt-0.5">{value}</p>
              </div>
            ))}
          </div>

          {/* Tags */}
          {rule.tags.length > 0 && (
            <div className="flex flex-wrap gap-1.5">
              {rule.tags.map(tag => (
                <span key={tag} className="px-2 py-1 rounded text-xs bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400">
                  {tag}
                </span>
              ))}
            </div>
          )}

          {/* Conditions summary */}
          <div className="rounded-xl border border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900 p-4">
            <p className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-2">Conditions</p>
            <pre className="text-xs font-mono text-gray-700 dark:text-gray-300 overflow-auto max-h-48 whitespace-pre-wrap">
              {yamlStringify(rule.conditions)}
            </pre>
          </div>

          {/* Actions summary */}
          <div className="rounded-xl border border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900 p-4">
            <p className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-2">Actions</p>
            <pre className="text-xs font-mono text-gray-700 dark:text-gray-300 overflow-auto max-h-48 whitespace-pre-wrap">
              {yamlStringify(rule.actions)}
            </pre>
          </div>
        </div>
      )}

      {tab === 'test' && (
        <TestPanel ruleId={rule.id} autoRun />
      )}

      {tab === 'history' && (
        <div className="space-y-2">
          <p className="text-sm text-gray-500 dark:text-gray-400">All stored versions of this rule:</p>
          {versions?.map(v => (
            <div key={v} className={clsx(
              'flex items-center gap-3 px-4 py-2.5 rounded-lg border text-sm',
              v === rule.version
                ? 'border-brand-300 dark:border-brand-700 bg-brand-50 dark:bg-brand-950 text-brand-700 dark:text-brand-300'
                : 'border-gray-200 dark:border-gray-800 text-gray-600 dark:text-gray-400',
            )}>
              <span className="font-medium">v{v}</span>
              {v === rule.version && <span className="text-xs">(current)</span>}
            </div>
          ))}
        </div>
      )}

      {tab === 'source' && (
        <pre className="rounded-xl border border-gray-200 dark:border-gray-800 bg-gray-50 dark:bg-gray-900 p-4 text-xs font-mono text-gray-800 dark:text-gray-200 overflow-auto max-h-[65vh] whitespace-pre-wrap">
          {yamlStringify(rule)}
        </pre>
      )}
    </div>
  )
}
