/**
 * @axiom-rules/core integration tests
 * Run with: node --test tests/engine.test.js
 */
'use strict'

const { test } = require('node:test')
const assert   = require('node:assert/strict')
const path     = require('node:path')
const { AxiomEngine, validateRule } = require('../index.js')

// ---------------------------------------------------------------------------
// Sample rule YAML
// ---------------------------------------------------------------------------

const LOAN_RULE_YAML = `
ars_version: 1
id: loan-eligibility-check
name: Loan Eligibility Check
version: 1
priority: 10
enabled: true
tags: [lending]
conditions:
  all:
    - field: applicant.credit_score
      operator: gte
      value: 650
    - field: applicant.annual_income
      operator: gte
      value: 30000
    - field: applicant.existing_debt_ratio
      operator: lte
      value: 0.4
actions:
  - type: set
    field: result.eligible
    value: true
  - type: set
    field: result.max_loan_amount
    value: "{{ applicant.annual_income * 3 }}"
  - type: tag
    value: standard-loan-approved
`

const PREMIUM_RULE_YAML = `
ars_version: 1
id: premium-discount
name: Premium Discount
version: 1
priority: 5
enabled: true
conditions:
  all:
    - field: user.plan
      operator: eq
      value: premium
    - field: order.total
      operator: gte
      value: 100
actions:
  - type: set
    field: discount.percentage
    value: 15
  - type: tag
    value: premium-discount
`

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

test('evaluate — loan eligibility match', () => {
  const engine = new AxiomEngine()
  engine.loadRuleYaml(LOAN_RULE_YAML)

  const resp = engine.evaluate({
    rule_id:  'loan-eligibility-check',
    strategy: 'first_match',
    context: {
      applicant: { credit_score: 720, annual_income: 60000, existing_debt_ratio: 0.2 }
    }
  })

  assert.equal(resp.matched, true)
  assert.deepEqual(resp.matched_rules, ['loan-eligibility-check'])
  assert.deepEqual(resp.tags, ['standard-loan-approved'])
  assert.equal(resp.output_context.result.eligible, true)
  assert.equal(resp.output_context.result.max_loan_amount, 180000)
})

test('evaluate — loan eligibility no match (low credit)', () => {
  const engine = new AxiomEngine()
  engine.loadRuleYaml(LOAN_RULE_YAML)

  const resp = engine.evaluate({
    rule_id: 'loan-eligibility-check',
    context: { applicant: { credit_score: 580, annual_income: 60000, existing_debt_ratio: 0.2 } }
  })

  assert.equal(resp.matched, false)
  assert.deepEqual(resp.matched_rules, [])
})

test('evaluate — all_match strategy returns multiple rules', () => {
  const engine = new AxiomEngine()
  engine.loadRuleYaml(LOAN_RULE_YAML)
  engine.loadRuleYaml(PREMIUM_RULE_YAML)

  const resp = engine.evaluate({
    strategy: 'all_match',
    context: {
      applicant: { credit_score: 720, annual_income: 60000, existing_debt_ratio: 0.2 },
      user:      { plan: 'premium' },
      order:     { total: 150 }
    }
  })

  assert.equal(resp.matched, true)
  assert.ok(resp.matched_rules.includes('loan-eligibility-check'))
  assert.ok(resp.matched_rules.includes('premium-discount'))
})

test('evaluate — trace contains condition details', () => {
  const engine = new AxiomEngine()
  engine.loadRuleYaml(LOAN_RULE_YAML)

  const resp = engine.evaluate({
    rule_id: 'loan-eligibility-check',
    context: { applicant: { credit_score: 720, annual_income: 60000, existing_debt_ratio: 0.2 } }
  })

  assert.ok(resp.trace.rules.length > 0)
  const ruleTrace = resp.trace.rules[0]
  assert.equal(ruleTrace.rule_id, 'loan-eligibility-check')
  assert.equal(ruleTrace.matched, true)
  assert.ok(ruleTrace.conditions.length > 0)
  assert.equal(ruleTrace.conditions[0].passed, true)
})

test('loadRuleJson — accepts JSON string', () => {
  const engine = new AxiomEngine()
  const ruleJson = JSON.stringify({
    ars_version: 1, id: 'json-rule', name: 'JSON Rule',
    version: 1, enabled: true,
    conditions: { all: [{ field: 'x', operator: 'eq', value: 1 }] },
    actions: [{ type: 'tag', value: 'hit' }]
  })
  engine.loadRuleJson(ruleJson)

  const resp = engine.evaluate({ rule_id: 'json-rule', context: { x: 1 } })
  assert.equal(resp.matched, true)
})

test('validateRule — valid rule returns valid:true', () => {
  const result = validateRule(LOAN_RULE_YAML)
  assert.equal(result.valid, true)
})

test('validateRule — wrong ars_version returns error', () => {
  const bad = LOAN_RULE_YAML.replace('ars_version: 1', 'ars_version: 99')
  const result = validateRule(bad)
  assert.equal(result.valid, false)
  assert.ok(result.error.includes('99'))
})

test('evaluateAsync — promise resolves', async () => {
  const engine = new AxiomEngine()
  engine.loadRuleYaml(LOAN_RULE_YAML)
  const resp = await engine.evaluateAsync({
    rule_id: 'loan-eligibility-check',
    context: { applicant: { credit_score: 720, annual_income: 60000, existing_debt_ratio: 0.2 } }
  })
  assert.equal(resp.matched, true)
})
