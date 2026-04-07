import { useState } from 'react'
import { Eye, EyeOff, RefreshCw, Key, Plus, Trash2 } from 'lucide-react'
import { useSettingsStore } from '../store/settings'
import { useHealth, useKeys, useCreateKey, useRevokeKey } from '../api/hooks'
import type { Role } from '../store/settings'
import { getHealth } from '../api/client'

export default function SettingsPage() {
  const { serverUrl, apiKey, role, setServerUrl, setApiKey, setRole } = useSettingsStore()
  const [urlDraft, setUrlDraft]     = useState(serverUrl)
  const [keyDraft, setKeyDraft]     = useState(apiKey)
  const [showKey, setShowKey]       = useState(false)
  const [testing, setTesting]       = useState(false)
  const [testResult, setTestResult] = useState<string | null>(null)
  const [newKeyRole, setNewKeyRole] = useState<'admin' | 'editor' | 'viewer'>('editor')
  const [newKeyDesc, setNewKeyDesc] = useState('')
  const [createdKey, setCreatedKey] = useState<string | null>(null)

  const { refetch: refetchHealth } = useHealth()
  const { data: keys, isLoading: keysLoading } = useKeys()
  const createKey = useCreateKey()
  const revokeKey = useRevokeKey()

  async function testConnection() {
    setTesting(true)
    setTestResult(null)
    // Temporarily apply draft settings for the test
    const prev = { serverUrl: useSettingsStore.getState().serverUrl, apiKey: useSettingsStore.getState().apiKey }
    useSettingsStore.setState({ serverUrl: urlDraft, apiKey: keyDraft })
    try {
      const h = await getHealth()
      setTestResult(`✓ Connected — ${h.status}`)
      setServerUrl(urlDraft)
      setApiKey(keyDraft)
      await refetchHealth()
      // Try to infer role from keys list (heuristic: if 403 we're viewer or no key)
      setRole(null) // will be populated when keys are listed
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e)
      setTestResult(`✗ ${msg}`)
      useSettingsStore.setState(prev) // revert on failure
    } finally {
      setTesting(false)
    }
  }

  async function handleCreateKey() {
    const res = await createKey.mutateAsync({ role: newKeyRole, description: newKeyDesc || undefined })
    setCreatedKey(res.key)
    setNewKeyDesc('')
  }

  return (
    <div className="max-w-2xl mx-auto px-6 py-8 space-y-8">
      <h1 className="text-2xl font-bold text-gray-900 dark:text-white">Settings</h1>

      {/* Connection */}
      <section className="bg-white dark:bg-gray-900 rounded-xl border border-gray-200 dark:border-gray-800 p-6 space-y-4">
        <h2 className="font-semibold text-gray-900 dark:text-white">Server connection</h2>

        <div className="space-y-3">
          <label className="block">
            <span className="text-sm font-medium text-gray-700 dark:text-gray-300">Server URL</span>
            <input
              type="url"
              value={urlDraft}
              onChange={e => setUrlDraft(e.target.value)}
              className="mt-1 block w-full rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 px-3 py-2 text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-brand-500"
              placeholder="http://localhost:8080"
            />
          </label>

          <label className="block">
            <span className="text-sm font-medium text-gray-700 dark:text-gray-300">API Key</span>
            <div className="mt-1 flex gap-2">
              <div className="relative flex-1">
                <input
                  type={showKey ? 'text' : 'password'}
                  value={keyDraft}
                  onChange={e => setKeyDraft(e.target.value)}
                  className="block w-full rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 px-3 py-2 pr-10 text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-brand-500"
                  placeholder="axm-…"
                />
                <button
                  type="button"
                  onClick={() => setShowKey(v => !v)}
                  className="absolute inset-y-0 right-2 flex items-center text-gray-400 hover:text-gray-600"
                >
                  {showKey ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                </button>
              </div>
              <button
                onClick={testConnection}
                disabled={testing}
                className="flex items-center gap-1.5 px-4 py-2 rounded-lg bg-brand-600 text-white text-sm font-medium hover:bg-brand-700 disabled:opacity-50 transition-colors"
              >
                <RefreshCw className={`w-3.5 h-3.5 ${testing ? 'animate-spin' : ''}`} />
                {testing ? 'Testing…' : 'Connect'}
              </button>
            </div>
          </label>

          {testResult && (
            <p className={`text-sm ${testResult.startsWith('✓') ? 'text-green-600 dark:text-green-400' : 'text-red-600 dark:text-red-400'}`}>
              {testResult}
            </p>
          )}
        </div>

        {role && (
          <p className="text-sm text-gray-500 dark:text-gray-400">
            Authenticated as <span className="font-medium text-gray-700 dark:text-gray-300 capitalize">{role}</span>
          </p>
        )}
      </section>

      {/* API key management — admin only */}
      {(role === 'admin' || role === null) && (
        <section className="bg-white dark:bg-gray-900 rounded-xl border border-gray-200 dark:border-gray-800 p-6 space-y-4">
          <h2 className="font-semibold text-gray-900 dark:text-white flex items-center gap-2">
            <Key className="w-4 h-4" /> API Keys
          </h2>

          {/* Create key */}
          <div className="flex gap-2">
            <select
              value={newKeyRole}
              onChange={e => setNewKeyRole(e.target.value as typeof newKeyRole)}
              className="rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 px-3 py-2 text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-brand-500"
            >
              <option value="admin">Admin</option>
              <option value="editor">Editor</option>
              <option value="viewer">Viewer</option>
            </select>
            <input
              type="text"
              value={newKeyDesc}
              onChange={e => setNewKeyDesc(e.target.value)}
              placeholder="Description (optional)"
              className="flex-1 rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 px-3 py-2 text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-brand-500"
            />
            <button
              onClick={handleCreateKey}
              disabled={createKey.isPending}
              className="flex items-center gap-1.5 px-3 py-2 rounded-lg bg-brand-600 text-white text-sm font-medium hover:bg-brand-700 disabled:opacity-50 transition-colors"
            >
              <Plus className="w-4 h-4" /> Create
            </button>
          </div>

          {/* Newly created key — show once */}
          {createdKey && (
            <div className="rounded-lg bg-green-50 dark:bg-green-950 border border-green-200 dark:border-green-800 p-3">
              <p className="text-xs font-medium text-green-800 dark:text-green-300 mb-1">New key (shown once — copy now):</p>
              <code className="text-xs font-mono text-green-900 dark:text-green-200 break-all">{createdKey}</code>
              <button onClick={() => setCreatedKey(null)} className="ml-2 text-xs text-green-600 underline">Dismiss</button>
            </div>
          )}

          {/* Key list */}
          {keysLoading ? (
            <p className="text-sm text-gray-400">Loading…</p>
          ) : (
            <ul className="divide-y divide-gray-100 dark:divide-gray-800">
              {keys?.map(k => (
                <li key={k.id} className="flex items-center justify-between py-2">
                  <div>
                    <span className="text-sm font-mono text-gray-700 dark:text-gray-300">{k.id}</span>
                    <span className={`ml-2 text-xs px-1.5 py-0.5 rounded font-medium ${
                      k.role === 'admin' ? 'bg-red-100 text-red-700 dark:bg-red-950 dark:text-red-300'
                      : k.role === 'editor' ? 'bg-blue-100 text-blue-700 dark:bg-blue-950 dark:text-blue-300'
                      : 'bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400'
                    }`}>{k.role}</span>
                    {k.description && <span className="ml-2 text-xs text-gray-400">{k.description}</span>}
                  </div>
                  <button
                    onClick={() => revokeKey.mutate(k.id)}
                    className="text-red-500 hover:text-red-700 p-1"
                    title="Revoke key"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>
      )}
    </div>
  )
}
