import { Link } from 'react-router-dom'
import { Tag, ChevronRight, CircleDot } from 'lucide-react'
import clsx from 'clsx'
import type { Rule } from '../types/ars'

interface Props {
  rule: Rule
}

export default function RuleCard({ rule }: Props) {
  return (
    <Link
      to={`/rules/${encodeURIComponent(rule.id)}`}
      className="group block rounded-xl border border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900 p-4 hover:border-brand-300 dark:hover:border-brand-700 hover:shadow-sm transition-all"
    >
      <div className="flex items-start justify-between gap-3">
        <div className="flex-1 min-w-0">
          {/* Header */}
          <div className="flex items-center gap-2 mb-1">
            <CircleDot className={clsx('w-3 h-3 shrink-0', rule.enabled ? 'text-green-500' : 'text-gray-300 dark:text-gray-600')} />
            <span className="text-xs font-mono text-gray-400 truncate">{rule.id}</span>
            <span className="ml-auto text-xs text-gray-400">p{rule.priority}</span>
          </div>
          {/* Name */}
          <p className="text-sm font-semibold text-gray-900 dark:text-white group-hover:text-brand-600 dark:group-hover:text-brand-400 truncate">
            {rule.name}
          </p>
          {/* Description */}
          {rule.description && (
            <p className="mt-0.5 text-xs text-gray-500 dark:text-gray-400 line-clamp-2">{rule.description}</p>
          )}
          {/* Tags */}
          {rule.tags.length > 0 && (
            <div className="mt-2 flex flex-wrap gap-1">
              {rule.tags.map(tag => (
                <span key={tag} className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-xs bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400">
                  <Tag className="w-2.5 h-2.5" />{tag}
                </span>
              ))}
            </div>
          )}
        </div>
        <ChevronRight className="w-4 h-4 text-gray-300 dark:text-gray-600 group-hover:text-brand-400 shrink-0 mt-0.5 transition-colors" />
      </div>

      {/* Footer */}
      <div className="mt-3 flex items-center gap-3 text-xs text-gray-400">
        <span>v{rule.version}</span>
        <span>{rule.actions.length} action{rule.actions.length !== 1 ? 's' : ''}</span>
        <span className={clsx('ml-auto font-medium', rule.enabled ? 'text-green-600 dark:text-green-400' : 'text-gray-400')}>
          {rule.enabled ? 'Enabled' : 'Disabled'}
        </span>
      </div>
    </Link>
  )
}
