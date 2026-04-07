import { useState, useMemo } from 'react'
import { Link } from 'react-router-dom'
import { Plus, Search, LayoutGrid, List, Tag, SlidersHorizontal } from 'lucide-react'
import { useRules } from '../api/hooks'
import { useSettingsStore } from '../store/settings'
import RuleCard from '../components/RuleCard'
import type { Rule } from '../types/ars'
import clsx from 'clsx'

type ViewMode = 'card' | 'table'
type EnabledFilter = 'all' | 'enabled' | 'disabled'

export default function RulesPage() {
  const { role } = useSettingsStore()
  const { data: rules, isLoading, isError, refetch } = useRules()

  const [search, setSearch]           = useState('')
  const [tagFilter, setTagFilter]     = useState('')
  const [enabledFilter, setEnabled]   = useState<EnabledFilter>('all')
  const [viewMode, setViewMode]       = useState<ViewMode>('card')

  const allTags = useMemo(() => {
    const tags = new Set<string>()
    rules?.forEach(r => r.tags.forEach(t => tags.add(t)))
    return [...tags].sort()
  }, [rules])

  const filtered = useMemo<Rule[]>(() => {
    if (!rules) return []
    return rules.filter(r => {
      if (search && !r.name.toLowerCase().includes(search.toLowerCase()) &&
          !r.id.toLowerCase().includes(search.toLowerCase())) return false
      if (tagFilter && !r.tags.includes(tagFilter)) return false
      if (enabledFilter === 'enabled'  && !r.enabled) return false
      if (enabledFilter === 'disabled' && r.enabled)  return false
      return true
    })
  }, [rules, search, tagFilter, enabledFilter])

  const canEdit = role === 'admin' || role === 'editor'

  return (
    <div className="px-6 py-6">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-white">Rules</h1>
          {rules && (
            <p className="text-sm text-gray-500 dark:text-gray-400 mt-0.5">
              {filtered.length} of {rules.length} rules
            </p>
          )}
        </div>
        {canEdit && (
          <Link
            to="/rules/new"
            className="flex items-center gap-2 px-4 py-2 rounded-lg bg-brand-600 text-white text-sm font-medium hover:bg-brand-700 transition-colors"
          >
            <Plus className="w-4 h-4" /> New rule
          </Link>
        )}
      </div>

      {/* Filters */}
      <div className="flex flex-wrap items-center gap-3 mb-5">
        {/* Search */}
        <div className="relative flex-1 min-w-48">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
          <input
            type="search"
            value={search}
            onChange={e => setSearch(e.target.value)}
            placeholder="Search by name or ID…"
            className="w-full pl-9 pr-3 py-2 rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-brand-500"
          />
        </div>

        {/* Tag filter */}
        <div className="flex items-center gap-1.5">
          <Tag className="w-4 h-4 text-gray-400" />
          <select
            value={tagFilter}
            onChange={e => setTagFilter(e.target.value)}
            className="rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 px-3 py-2 text-sm text-gray-700 dark:text-gray-300 focus:outline-none focus:ring-2 focus:ring-brand-500"
          >
            <option value="">All tags</option>
            {allTags.map(t => <option key={t} value={t}>{t}</option>)}
          </select>
        </div>

        {/* Enabled filter */}
        <div className="flex items-center gap-1.5">
          <SlidersHorizontal className="w-4 h-4 text-gray-400" />
          <select
            value={enabledFilter}
            onChange={e => setEnabled(e.target.value as EnabledFilter)}
            className="rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 px-3 py-2 text-sm text-gray-700 dark:text-gray-300 focus:outline-none focus:ring-2 focus:ring-brand-500"
          >
            <option value="all">All</option>
            <option value="enabled">Enabled</option>
            <option value="disabled">Disabled</option>
          </select>
        </div>

        {/* View toggle */}
        <div className="flex rounded-lg border border-gray-300 dark:border-gray-700 overflow-hidden">
          {(['card', 'table'] as ViewMode[]).map(mode => (
            <button
              key={mode}
              onClick={() => setViewMode(mode)}
              className={clsx(
                'px-3 py-2 transition-colors',
                viewMode === mode
                  ? 'bg-brand-600 text-white'
                  : 'bg-white dark:bg-gray-900 text-gray-500 hover:bg-gray-50 dark:hover:bg-gray-800',
              )}
            >
              {mode === 'card' ? <LayoutGrid className="w-4 h-4" /> : <List className="w-4 h-4" />}
            </button>
          ))}
        </div>
      </div>

      {/* Content */}
      {isLoading && (
        <div className="flex justify-center py-16">
          <div className="w-6 h-6 border-2 border-brand-500 border-t-transparent rounded-full animate-spin" />
        </div>
      )}

      {isError && (
        <div className="rounded-xl border border-red-200 dark:border-red-900 bg-red-50 dark:bg-red-950 p-6 text-center">
          <p className="text-red-700 dark:text-red-300 text-sm">Failed to load rules.</p>
          <button onClick={() => refetch()} className="mt-2 text-sm text-red-600 underline">Retry</button>
        </div>
      )}

      {!isLoading && !isError && filtered.length === 0 && (
        <div className="text-center py-16 text-gray-400 dark:text-gray-500">
          {rules?.length === 0 ? (
            <>
              <p className="text-lg font-medium mb-2">No rules yet</p>
              {canEdit && (
                <Link to="/rules/new" className="text-brand-500 hover:text-brand-700 text-sm">
                  Create your first rule →
                </Link>
              )}
            </>
          ) : (
            <p className="text-sm">No rules match your filters.</p>
          )}
        </div>
      )}

      {!isLoading && !isError && filtered.length > 0 && (
        viewMode === 'card' ? (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
            {filtered.map(rule => <RuleCard key={rule.id} rule={rule} />)}
          </div>
        ) : (
          <div className="rounded-xl border border-gray-200 dark:border-gray-800 overflow-hidden">
            <table className="w-full text-sm">
              <thead className="bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
                <tr>
                  {['ID', 'Name', 'Priority', 'Version', 'Tags', 'Status'].map(h => (
                    <th key={h} className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                      {h}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100 dark:divide-gray-800 bg-white dark:bg-gray-900">
                {filtered.map(rule => (
                  <tr key={rule.id} className="hover:bg-gray-50 dark:hover:bg-gray-800 cursor-pointer">
                    <td className="px-4 py-3">
                      <Link to={`/rules/${encodeURIComponent(rule.id)}`} className="font-mono text-xs text-brand-600 dark:text-brand-400 hover:underline">
                        {rule.id}
                      </Link>
                    </td>
                    <td className="px-4 py-3 font-medium text-gray-900 dark:text-white">{rule.name}</td>
                    <td className="px-4 py-3 text-gray-500 dark:text-gray-400">{rule.priority}</td>
                    <td className="px-4 py-3 text-gray-500 dark:text-gray-400">v{rule.version}</td>
                    <td className="px-4 py-3">
                      <div className="flex flex-wrap gap-1">
                        {rule.tags.map(t => (
                          <span key={t} className="px-1.5 py-0.5 rounded text-xs bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400">{t}</span>
                        ))}
                      </div>
                    </td>
                    <td className="px-4 py-3">
                      <span className={clsx(
                        'px-2 py-0.5 rounded-full text-xs font-medium',
                        rule.enabled
                          ? 'bg-green-100 text-green-700 dark:bg-green-950 dark:text-green-400'
                          : 'bg-gray-100 text-gray-500 dark:bg-gray-800 dark:text-gray-400',
                      )}>
                        {rule.enabled ? 'Enabled' : 'Disabled'}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )
      )}
    </div>
  )
}
