import { useState } from 'react'
import { Plus, ChevronDown, ChevronRight, FlaskConical, X, Check } from 'lucide-react'
import { useRulesets, useRules, useCreateRuleset, useUpdateRuleset } from '../api/hooks'
import { useSettingsStore } from '../store/settings'
import TestPanel from '../components/TestPanel'
import type { Ruleset } from '../types/ars'
import clsx from 'clsx'

// ---------------------------------------------------------------------------
// Ruleset editor modal
// ---------------------------------------------------------------------------

interface EditorProps {
  initial?: Ruleset
  ruleIds: string[]
  onSave: (rs: Ruleset) => void
  onClose: () => void
}

function RulesetEditor({ initial, ruleIds, onSave, onClose }: EditorProps) {
  const [name, setName]         = useState(initial?.name ?? '')
  const [desc, setDesc]         = useState(initial?.description ?? '')
  const [members, setMembers]   = useState<string[]>(initial?.rule_ids ?? [])

  function toggle(id: string) {
    setMembers(m => m.includes(id) ? m.filter(x => x !== id) : [...m, id])
  }

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
      <div className="bg-white dark:bg-gray-900 rounded-xl shadow-2xl w-full max-w-lg max-h-[80vh] flex flex-col">
        <div className="flex items-center justify-between px-5 py-4 border-b border-gray-200 dark:border-gray-800">
          <h2 className="font-semibold text-gray-900 dark:text-white">{initial ? 'Edit ruleset' : 'New ruleset'}</h2>
          <button onClick={onClose} className="text-gray-400 hover:text-gray-600"><X className="w-4 h-4" /></button>
        </div>

        <div className="flex-1 overflow-y-auto px-5 py-4 space-y-4">
          <label className="block">
            <span className="text-xs font-medium text-gray-700 dark:text-gray-300">Name</span>
            <input type="text" value={name} onChange={e => setName(e.target.value)} placeholder="my-ruleset" disabled={!!initial} className="mt-1 block w-full rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 px-3 py-2 text-sm font-mono text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-brand-500 disabled:opacity-60" />
          </label>
          <label className="block">
            <span className="text-xs font-medium text-gray-700 dark:text-gray-300">Description</span>
            <textarea value={desc} onChange={e => setDesc(e.target.value)} rows={2} className="mt-1 block w-full rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 px-3 py-2 text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-brand-500 resize-none" />
          </label>

          <div>
            <p className="text-xs font-medium text-gray-700 dark:text-gray-300 mb-2">Rule membership</p>
            <div className="space-y-1 max-h-48 overflow-y-auto">
              {ruleIds.map(id => (
                <label key={id} className="flex items-center gap-2.5 px-3 py-2 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={members.includes(id)}
                    onChange={() => toggle(id)}
                    className="rounded border-gray-300 text-brand-600 focus:ring-brand-500"
                  />
                  <span className="text-sm font-mono text-gray-700 dark:text-gray-300">{id}</span>
                  {members.includes(id) && (
                    <span className="ml-auto text-xs text-gray-400">#{members.indexOf(id) + 1}</span>
                  )}
                </label>
              ))}
            </div>
          </div>
        </div>

        <div className="px-5 py-4 border-t border-gray-200 dark:border-gray-800 flex justify-end gap-2">
          <button onClick={onClose} className="px-4 py-2 rounded-lg border border-gray-300 dark:border-gray-700 text-sm text-gray-600 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800">
            Cancel
          </button>
          <button
            onClick={() => onSave({ name: name.trim(), description: desc.trim() || undefined, rule_ids: members })}
            disabled={!name.trim()}
            className="flex items-center gap-2 px-4 py-2 rounded-lg bg-brand-600 text-white text-sm font-medium hover:bg-brand-700 disabled:opacity-50"
          >
            <Check className="w-4 h-4" /> Save
          </button>
        </div>
      </div>
    </div>
  )
}

// ---------------------------------------------------------------------------
// Ruleset card
// ---------------------------------------------------------------------------

interface CardProps {
  rs: Ruleset
  ruleIds: string[]
  canEdit: boolean
  onEdit: () => void
}

function RulesetCard({ rs, canEdit, onEdit }: CardProps) {
  const [open, setOpen]     = useState(false)
  const [testing, setTest]  = useState(false)

  return (
    <div className="bg-white dark:bg-gray-900 rounded-xl border border-gray-200 dark:border-gray-800 overflow-hidden">
      <div className="flex items-center gap-3 px-4 py-3">
        <button onClick={() => setOpen(v => !v)} className="text-gray-400">
          {open ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
        </button>
        <div className="flex-1 min-w-0">
          <p className="font-mono text-sm font-medium text-gray-900 dark:text-white truncate">{rs.name}</p>
          {rs.description && <p className="text-xs text-gray-400 truncate">{rs.description}</p>}
        </div>
        <span className="text-xs text-gray-400">{rs.rule_ids.length} rules</span>
        <div className="flex gap-2">
          <button
            onClick={() => setTest(v => !v)}
            className={clsx('flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-xs font-medium transition-colors', testing ? 'bg-brand-600 text-white' : 'border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-50 dark:hover:bg-gray-800')}
          >
            <FlaskConical className="w-3.5 h-3.5" /> Test
          </button>
          {canEdit && (
            <button onClick={onEdit} className="px-2.5 py-1.5 rounded-lg text-xs font-medium border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-50 dark:hover:bg-gray-800">
              Edit
            </button>
          )}
        </div>
      </div>

      {open && (
        <div className="border-t border-gray-100 dark:border-gray-800 px-4 py-3">
          <div className="flex flex-wrap gap-1.5">
            {rs.rule_ids.map((id, i) => (
              <span key={id} className="inline-flex items-center gap-1 px-2 py-1 rounded text-xs bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400">
                <span className="text-gray-300 dark:text-gray-600 font-mono text-[10px]">{i + 1}.</span> {id}
              </span>
            ))}
          </div>
        </div>
      )}

      {testing && (
        <div className="border-t border-gray-100 dark:border-gray-800 px-4 py-4">
          <TestPanel rulesetName={rs.name} autoRun />
        </div>
      )}
    </div>
  )
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function RulesetsPage() {
  const { role } = useSettingsStore()
  const { data: rulesets, isLoading } = useRulesets()
  const { data: rules } = useRules()
  const createRuleset = useCreateRuleset()
  const [editing, setEditing] = useState<Ruleset | null | 'new'>(null)

  const ruleIds = rules?.map(r => r.id) ?? []
  const canEdit = role === 'admin' || role === 'editor'

  // Determine the name of the ruleset being edited
  const editingName = editing && editing !== 'new' ? editing.name : ''
  const updateRuleset = useUpdateRuleset(editingName)

  async function handleSave(rs: Ruleset) {
    if (editing === 'new') {
      await createRuleset.mutateAsync(rs)
    } else {
      await updateRuleset.mutateAsync(rs)
    }
    setEditing(null)
  }

  return (
    <div className="px-6 py-6">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">Rulesets</h1>
        {canEdit && (
          <button
            onClick={() => setEditing('new')}
            className="flex items-center gap-2 px-4 py-2 rounded-lg bg-brand-600 text-white text-sm font-medium hover:bg-brand-700 transition-colors"
          >
            <Plus className="w-4 h-4" /> New ruleset
          </button>
        )}
      </div>

      {isLoading && (
        <div className="flex justify-center py-12">
          <div className="w-6 h-6 border-2 border-brand-500 border-t-transparent rounded-full animate-spin" />
        </div>
      )}

      <div className="space-y-3">
        {rulesets?.map(rs => (
          <RulesetCard
            key={rs.name}
            rs={rs}
            ruleIds={ruleIds}
            canEdit={canEdit}
            onEdit={() => setEditing(rs)}
          />
        ))}
      </div>

      {!isLoading && rulesets?.length === 0 && (
        <div className="text-center py-16 text-gray-400 dark:text-gray-500">
          <p className="text-lg font-medium mb-2">No rulesets yet</p>
          {canEdit && <button onClick={() => setEditing('new')} className="text-brand-500 text-sm">Create one →</button>}
        </div>
      )}

      {editing !== null && (
        <RulesetEditor
          initial={editing === 'new' ? undefined : editing}
          ruleIds={ruleIds}
          onSave={handleSave}
          onClose={() => setEditing(null)}
        />
      )}
    </div>
  )
}
