import { useSettingsStore } from '../store/settings'
import type {
  Rule, Ruleset, EvaluationRequest, EvaluationResponse, ApiKey,
} from '../types/ars'

// ---------------------------------------------------------------------------
// Base fetch helper
// ---------------------------------------------------------------------------

async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const { serverUrl, apiKey } = useSettingsStore.getState()
  const url = `${serverUrl.replace(/\/$/, '')}${path}`

  const res = await fetch(url, {
    ...init,
    headers: {
      'Content-Type': 'application/json',
      ...(apiKey ? { 'X-Axiom-Key': apiKey } : {}),
      ...init?.headers,
    },
  })

  if (!res.ok) {
    const body = await res.text()
    let message = `${res.status} ${res.statusText}`
    try { message = JSON.parse(body).error ?? message } catch { /* ignore */ }
    throw new ApiError(res.status, message)
  }

  // 204 No Content
  if (res.status === 204) return undefined as unknown as T
  return res.json() as Promise<T>
}

export class ApiError extends Error {
  constructor(public readonly status: number, message: string) {
    super(message)
    this.name = 'ApiError'
  }
}

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

export async function getHealth(): Promise<{ status: string }> {
  return apiFetch('/health')
}

// ---------------------------------------------------------------------------
// Rules
// ---------------------------------------------------------------------------

export interface RuleListParams {
  tag?: string
  enabled?: boolean
}

export async function listRules(params: RuleListParams = {}): Promise<Rule[]> {
  const q = new URLSearchParams()
  if (params.tag)            q.set('tag', params.tag)
  if (params.enabled != null) q.set('enabled', String(params.enabled))
  const qs = q.toString() ? `?${q}` : ''
  return apiFetch(`/v1/rules${qs}`)
}

export async function getRule(id: string): Promise<Rule> {
  return apiFetch(`/v1/rules/${encodeURIComponent(id)}`)
}

export async function createRule(rule: Omit<Rule, 'version'>): Promise<Rule> {
  return apiFetch('/v1/rules', { method: 'POST', body: JSON.stringify(rule) })
}

export async function updateRule(id: string, rule: Rule): Promise<Rule> {
  return apiFetch(`/v1/rules/${encodeURIComponent(id)}`, {
    method: 'PUT',
    body: JSON.stringify(rule),
  })
}

export async function patchRule(id: string, patch: Partial<Rule>): Promise<Rule> {
  return apiFetch(`/v1/rules/${encodeURIComponent(id)}`, {
    method: 'PATCH',
    body: JSON.stringify(patch),
  })
}

export async function deleteRule(id: string): Promise<void> {
  return apiFetch(`/v1/rules/${encodeURIComponent(id)}`, { method: 'DELETE' })
}

export async function listVersions(id: string): Promise<number[]> {
  return apiFetch(`/v1/rules/${encodeURIComponent(id)}/versions`)
}

// ---------------------------------------------------------------------------
// Rulesets
// ---------------------------------------------------------------------------

export async function listRulesets(): Promise<Ruleset[]> {
  return apiFetch('/v1/rulesets')
}

export async function getRuleset(name: string): Promise<Ruleset> {
  return apiFetch(`/v1/rulesets/${encodeURIComponent(name)}`)
}

export async function createRuleset(rs: Ruleset): Promise<Ruleset> {
  return apiFetch('/v1/rulesets', { method: 'POST', body: JSON.stringify(rs) })
}

export async function updateRuleset(name: string, rs: Ruleset): Promise<Ruleset> {
  return apiFetch(`/v1/rulesets/${encodeURIComponent(name)}`, {
    method: 'PUT',
    body: JSON.stringify(rs),
  })
}

// ---------------------------------------------------------------------------
// Evaluate
// ---------------------------------------------------------------------------

export async function evaluate(req: EvaluationRequest): Promise<EvaluationResponse> {
  return apiFetch('/v1/evaluate', { method: 'POST', body: JSON.stringify(req) })
}

export async function evaluateBatch(
  reqs: EvaluationRequest[],
): Promise<{ results: EvaluationResponse[] }> {
  return apiFetch('/v1/evaluate/batch', { method: 'POST', body: JSON.stringify(reqs) })
}

// ---------------------------------------------------------------------------
// API keys
// ---------------------------------------------------------------------------

export async function listKeys(): Promise<ApiKey[]> {
  return apiFetch('/v1/keys')
}

export interface CreateKeyRequest {
  role: 'admin' | 'editor' | 'viewer'
  description?: string
}

export interface CreateKeyResponse {
  id: string
  key: string   // plaintext — shown once
  role: string
}

export async function createKey(req: CreateKeyRequest): Promise<CreateKeyResponse> {
  return apiFetch('/v1/keys', { method: 'POST', body: JSON.stringify(req) })
}

export async function revokeKey(id: string): Promise<void> {
  return apiFetch(`/v1/keys/${encodeURIComponent(id)}`, { method: 'DELETE' })
}

// ---------------------------------------------------------------------------
// Import / export
// ---------------------------------------------------------------------------

export async function importBundle(bundle: unknown): Promise<{ imported: number }> {
  return apiFetch('/v1/import', { method: 'POST', body: JSON.stringify(bundle) })
}

export async function exportBundle(): Promise<unknown> {
  return apiFetch('/v1/export')
}
