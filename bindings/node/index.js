'use strict'

// Load the platform-specific native addon built by NAPI-RS.
// The .node file is named axiom.<target>.node by napi-rs/cli.
const { AxiomEngine, validateRule: _validateRule } = require('./axiom.node')

/**
 * Wrapper that ensures evaluate() accepts an object (not a JSON string)
 * and returns a parsed EvaluationResponse object.
 */
class AxiomEngineWrapper {
  constructor() {
    this._inner = new AxiomEngine()
  }

  loadRuleYaml(yaml) {
    this._inner.loadRuleYaml(yaml)
    return this
  }

  loadRuleJson(json) {
    const s = typeof json === 'string' ? json : JSON.stringify(json)
    this._inner.loadRuleJson(s)
    return this
  }

  loadRuleFile(path) {
    this._inner.loadRuleFile(path)
    return this
  }

  loadBundle(path) {
    this._inner.loadBundle(path)
    return this
  }

  evaluate(request) {
    const reqJson = JSON.stringify(request)
    const respJson = this._inner.evaluate(reqJson)
    return JSON.parse(respJson)
  }

  /** Alias for evaluateAsync (currently synchronous; async wrapper for future use) */
  async evaluateAsync(request) {
    return this.evaluate(request)
  }

  validateRule(source, isJson = false) {
    return this._inner.validateRule(source, isJson)
  }
}

/**
 * Validate an ARS YAML or JSON string.
 * @returns {{ valid: true } | { valid: false, error: string }}
 */
function validateRule(yamlOrJson) {
  const err = _validateRule(yamlOrJson)
  return err == null ? { valid: true } : { valid: false, error: err }
}

module.exports = { AxiomEngine: AxiomEngineWrapper, validateRule }
