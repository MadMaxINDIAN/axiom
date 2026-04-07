import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import * as api from './client'
import type { Rule, Ruleset, EvaluationRequest } from '../types/ars'

// ---------------------------------------------------------------------------
// Rules
// ---------------------------------------------------------------------------

export function useRules(params?: api.RuleListParams) {
  return useQuery({
    queryKey: ['rules', params],
    queryFn: () => api.listRules(params),
  })
}

export function useRule(id: string) {
  return useQuery({
    queryKey: ['rules', id],
    queryFn: () => api.getRule(id),
    enabled: !!id,
  })
}

export function useRuleVersions(id: string) {
  return useQuery({
    queryKey: ['rules', id, 'versions'],
    queryFn: () => api.listVersions(id),
    enabled: !!id,
  })
}

export function useCreateRule() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (rule: Omit<Rule, 'version'>) => api.createRule(rule),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['rules'] }),
  })
}

export function useUpdateRule(id: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (rule: Rule) => api.updateRule(id, rule),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['rules'] }),
  })
}

export function usePatchRule(id: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (patch: Partial<Rule>) => api.patchRule(id, patch),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['rules'] }),
  })
}

export function useDeleteRule() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api.deleteRule(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['rules'] }),
  })
}

// ---------------------------------------------------------------------------
// Rulesets
// ---------------------------------------------------------------------------

export function useRulesets() {
  return useQuery({
    queryKey: ['rulesets'],
    queryFn: api.listRulesets,
  })
}

export function useRuleset(name: string) {
  return useQuery({
    queryKey: ['rulesets', name],
    queryFn: () => api.getRuleset(name),
    enabled: !!name,
  })
}

export function useCreateRuleset() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (rs: Ruleset) => api.createRuleset(rs),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['rulesets'] }),
  })
}

export function useUpdateRuleset(name: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (rs: Ruleset) => api.updateRuleset(name, rs),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['rulesets'] }),
  })
}

// ---------------------------------------------------------------------------
// Evaluate
// ---------------------------------------------------------------------------

export function useEvaluate() {
  return useMutation({
    mutationFn: (req: EvaluationRequest) => api.evaluate(req),
  })
}

// ---------------------------------------------------------------------------
// Health / connection
// ---------------------------------------------------------------------------

export function useHealth() {
  return useQuery({
    queryKey: ['health'],
    queryFn: api.getHealth,
    refetchInterval: 30_000,
    retry: false,
  })
}

// ---------------------------------------------------------------------------
// API keys
// ---------------------------------------------------------------------------

export function useKeys() {
  return useQuery({
    queryKey: ['keys'],
    queryFn: api.listKeys,
  })
}

export function useCreateKey() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (req: api.CreateKeyRequest) => api.createKey(req),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['keys'] }),
  })
}

export function useRevokeKey() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api.revokeKey(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['keys'] }),
  })
}
