import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import * as yaml from 'js-yaml';

// ---------------------------------------------------------------------------
// Activation
// ---------------------------------------------------------------------------

export function activate(context: vscode.ExtensionContext): void {
    const diagnostics = vscode.languages.createDiagnosticCollection('axiom');
    context.subscriptions.push(diagnostics);

    // Register commands
    context.subscriptions.push(
        vscode.commands.registerCommand(
            'axiom.validateCurrentFile',
            () => validateCurrentFile(diagnostics),
        ),
        vscode.commands.registerCommand(
            'axiom.openVisualBuilder',
            () => openVisualBuilder(context),
        ),
        vscode.commands.registerCommand(
            'axiom.previewRule',
            () => previewRuleJson(context),
        ),
    );

    // Validate on save
    context.subscriptions.push(
        vscode.workspace.onDidSaveTextDocument((doc) => {
            if (isAxiomDocument(doc)) {
                const cfg = vscode.workspace.getConfiguration('axiom');
                if (cfg.get<boolean>('validateOnSave', true)) {
                    validateDocument(doc, diagnostics);
                }
            }
        }),
    );

    // Clear diagnostics when file is closed
    context.subscriptions.push(
        vscode.workspace.onDidCloseTextDocument((doc) => {
            diagnostics.delete(doc.uri);
        }),
    );

    // Validate already-open ARS documents on activation
    vscode.workspace.textDocuments
        .filter(isAxiomDocument)
        .forEach((doc) => validateDocument(doc, diagnostics));

    // Register hover provider for operators
    context.subscriptions.push(
        vscode.languages.registerHoverProvider(
            [{ language: 'axiom-rule' }, { language: 'yaml', pattern: '**/{*.ars.yaml,*.ars.yml,bundle.yaml,bundle.yml}' }],
            new AxiomHoverProvider(),
        ),
    );

    // Register completion provider
    context.subscriptions.push(
        vscode.languages.registerCompletionItemProvider(
            [{ language: 'axiom-rule' }, { language: 'yaml', pattern: '**/{*.ars.yaml,*.ars.yml,bundle.yaml,bundle.yml}' }],
            new AxiomCompletionProvider(),
            ':', ' ', '-',
        ),
    );

    vscode.window.setStatusBarMessage('$(check) Axiom Rules extension active', 3000);
}

export function deactivate(): void {
    // Nothing to clean up
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function isAxiomDocument(doc: vscode.TextDocument): boolean {
    const name = path.basename(doc.fileName);
    return (
        doc.languageId === 'axiom-rule' ||
        name === 'bundle.yaml' ||
        name === 'bundle.yml' ||
        doc.fileName.endsWith('.ars.yaml') ||
        doc.fileName.endsWith('.ars.yml') ||
        doc.fileName.endsWith('.ars.json')
    );
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

async function validateCurrentFile(
    diagnostics: vscode.DiagnosticCollection,
): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showWarningMessage('Axiom: No active editor.');
        return;
    }
    if (!isAxiomDocument(editor.document)) {
        vscode.window.showWarningMessage('Axiom: Active file is not an ARS file.');
        return;
    }
    await validateDocument(editor.document, diagnostics);
}

async function validateDocument(
    doc: vscode.TextDocument,
    collection: vscode.DiagnosticCollection,
): Promise<void> {
    const text = doc.getText();
    const diags: vscode.Diagnostic[] = [];

    // Fast local structural validation (YAML parse + key checks)
    const localDiags = localValidate(text, doc);
    diags.push(...localDiags);

    // Attempt server-side validation if configured
    const cfg = vscode.workspace.getConfiguration('axiom');
    const serverUrl = cfg.get<string>('serverUrl', 'http://localhost:8080');
    const apiKey = cfg.get<string>('apiKey', '');

    if (serverUrl && diags.length === 0) {
        const serverDiags = await serverValidate(text, doc, serverUrl, apiKey);
        diags.push(...serverDiags);
    }

    collection.set(doc.uri, diags);

    if (diags.length === 0) {
        vscode.window.setStatusBarMessage('$(check) Axiom: Valid', 4000);
    } else {
        vscode.window.setStatusBarMessage(`$(error) Axiom: ${diags.length} issue(s)`, 4000);
    }
}

function localValidate(
    text: string,
    doc: vscode.TextDocument,
): vscode.Diagnostic[] {
    const diags: vscode.Diagnostic[] = [];

    // Try to parse YAML/JSON
    let parsed: unknown;
    try {
        if (doc.fileName.endsWith('.json')) {
            parsed = JSON.parse(text);
        } else {
            parsed = yaml.load(text);
        }
    } catch (e: unknown) {
        const msg = e instanceof Error ? e.message : String(e);
        diags.push(
            new vscode.Diagnostic(
                new vscode.Range(0, 0, 0, 0),
                `Parse error: ${msg}`,
                vscode.DiagnosticSeverity.Error,
            ),
        );
        return diags;
    }

    if (typeof parsed !== 'object' || parsed === null) {
        diags.push(
            makeDiag(doc, 0, 'File must be a YAML/JSON object.', vscode.DiagnosticSeverity.Error),
        );
        return diags;
    }

    const obj = parsed as Record<string, unknown>;

    // Check ars_version
    if (!('ars_version' in obj)) {
        diags.push(
            makeDiag(doc, findLineOf(text, 'ars_version', 0), 'Missing required field: ars_version', vscode.DiagnosticSeverity.Error),
        );
    } else if (obj['ars_version'] !== '1.0') {
        diags.push(
            makeDiag(doc, findLineOf(text, 'ars_version', 0), `ars_version must be "1.0", got "${obj['ars_version']}"`, vscode.DiagnosticSeverity.Warning),
        );
    }

    const isBundle = 'rules' in obj && Array.isArray(obj['rules']);

    if (isBundle) {
        validateBundle(obj, text, doc, diags);
    } else {
        validateSingleRule(obj, text, doc, diags);
    }

    return diags;
}

function validateSingleRule(
    obj: Record<string, unknown>,
    text: string,
    doc: vscode.TextDocument,
    diags: vscode.Diagnostic[],
): void {
    if (!('id' in obj)) {
        diags.push(makeDiag(doc, 0, 'Missing required field: id', vscode.DiagnosticSeverity.Error));
    } else if (typeof obj['id'] !== 'string' || !/^[a-zA-Z][a-zA-Z0-9_.\\-]*$/.test(obj['id'] as string)) {
        diags.push(
            makeDiag(
                doc,
                findLineOf(text, 'id:', 0),
                'Rule "id" must start with a letter and contain only letters, digits, underscores, hyphens, or dots.',
                vscode.DiagnosticSeverity.Warning,
            ),
        );
    }
    if (!('conditions' in obj)) {
        diags.push(makeDiag(doc, findLineOf(text, 'conditions', 0), 'Missing required field: conditions', vscode.DiagnosticSeverity.Error));
    }
    if (!('actions' in obj) || !Array.isArray(obj['actions']) || (obj['actions'] as unknown[]).length === 0) {
        diags.push(makeDiag(doc, findLineOf(text, 'actions', 0), 'Rule must have at least one action.', vscode.DiagnosticSeverity.Error));
    }
}

function validateBundle(
    obj: Record<string, unknown>,
    text: string,
    doc: vscode.TextDocument,
    diags: vscode.Diagnostic[],
): void {
    const rules = obj['rules'] as unknown[];
    if (!rules || rules.length === 0) {
        diags.push(makeDiag(doc, findLineOf(text, 'rules:', 0), 'Bundle must contain at least one rule.', vscode.DiagnosticSeverity.Error));
        return;
    }

    const ruleIds = new Set<string>();
    rules.forEach((r, i) => {
        if (typeof r !== 'object' || r === null) return;
        const rule = r as Record<string, unknown>;
        const id = rule['id'] as string | undefined;
        if (!id) {
            diags.push(makeDiag(doc, 0, `Rule at index ${i} is missing "id".`, vscode.DiagnosticSeverity.Error));
        } else {
            if (ruleIds.has(id)) {
                diags.push(makeDiag(doc, findLineOf(text, id, 0), `Duplicate rule id: "${id}"`, vscode.DiagnosticSeverity.Error));
            }
            ruleIds.add(id);
        }
        if (!('conditions' in rule)) {
            diags.push(makeDiag(doc, 0, `Rule "${id ?? i}" is missing "conditions".`, vscode.DiagnosticSeverity.Error));
        }
        if (!('actions' in rule) || !Array.isArray(rule['actions']) || (rule['actions'] as unknown[]).length === 0) {
            diags.push(makeDiag(doc, 0, `Rule "${id ?? i}" must have at least one action.`, vscode.DiagnosticSeverity.Error));
        }
    });

    // Validate ruleset rule references
    if ('rulesets' in obj && Array.isArray(obj['rulesets'])) {
        (obj['rulesets'] as unknown[]).forEach((rs) => {
            if (typeof rs !== 'object' || rs === null) return;
            const ruleset = rs as Record<string, unknown>;
            if (Array.isArray(ruleset['rules'])) {
                (ruleset['rules'] as string[]).forEach((ref) => {
                    if (!ruleIds.has(ref)) {
                        diags.push(
                            makeDiag(
                                doc,
                                findLineOf(text, ref, 0),
                                `Ruleset "${ruleset['id'] ?? '?'}" references unknown rule: "${ref}"`,
                                vscode.DiagnosticSeverity.Warning,
                            ),
                        );
                    }
                });
            }
        });
    }
}

async function serverValidate(
    text: string,
    doc: vscode.TextDocument,
    serverUrl: string,
    apiKey: string,
): Promise<vscode.Diagnostic[]> {
    try {
        const url = `${serverUrl.replace(/\/$/, '')}/v1/validate`;
        const headers: Record<string, string> = { 'Content-Type': 'text/plain' };
        if (apiKey) headers['X-API-Key'] = apiKey;

        const resp = await fetch(url, {
            method: 'POST',
            headers,
            body: text,
            signal: AbortSignal.timeout(5000),
        });

        if (resp.ok) return [];

        const body = await resp.text().catch(() => '');
        let message = `Server validation failed (HTTP ${resp.status})`;
        try {
            const json = JSON.parse(body) as { error?: string };
            if (json.error) message = json.error;
        } catch {
            if (body) message = body;
        }

        return [makeDiag(doc, 0, message, vscode.DiagnosticSeverity.Error)];
    } catch {
        // Server unavailable — silently skip server validation
        return [];
    }
}

// ---------------------------------------------------------------------------
// Hover Provider
// ---------------------------------------------------------------------------

const OPERATOR_DOCS: Record<string, string> = {
    eq:          '**eq** — Equal to. `field == value`',
    ne:          '**ne** — Not equal to. `field != value`',
    gt:          '**gt** — Greater than. `field > value`',
    gte:         '**gte** — Greater than or equal to. `field >= value`',
    lt:          '**lt** — Less than. `field < value`',
    lte:         '**lte** — Less than or equal to. `field <= value`',
    in:          '**in** — Field value is in the list. `value: [a, b, c]`',
    not_in:      '**not_in** — Field value is NOT in the list.',
    contains:    '**contains** — String/array contains the value.',
    not_contains:'**not_contains** — String/array does not contain the value.',
    starts_with: '**starts_with** — String starts with value.',
    ends_with:   '**ends_with** — String ends with value.',
    matches:     '**matches** — Field matches the regex pattern in value.',
    exists:      '**exists** — Field key exists in the context (value ignored).',
    not_exists:  '**not_exists** — Field key does NOT exist in the context.',
    between:     '**between** — Field is within `[min, max]` inclusive. `value: [min, max]`',
    not_between: '**not_between** — Field is outside `[min, max]`.',
    is_null:     '**is_null** — Field value is null.',
    is_not_null: '**is_not_null** — Field value is not null.',
};

const ACTION_DOCS: Record<string, string> = {
    approve:   '**approve** — Marks the evaluation as approved.',
    decline:   '**decline** — Marks the evaluation as declined. Optional `reason:` field.',
    flag:      '**flag** — Flags the entity for review.',
    tag:       '**tag** — Adds a tag to the result. Requires `tag:` field.',
    set_value: '**set_value** — Sets a key in the output context. Requires `key:` and `value:`.',
    call_rule: '**call_rule** — Invokes another rule by ID. Requires `rule_id:`.',
    trigger:   '**trigger** — Emits a named event. Requires `event:`.',
    log:       '**log** — Logs a message. Requires `message:`.',
    notify:    '**notify** — Sends a notification. Requires `channel:` and `message:`.',
    set_score: '**set_score** — Adds to the scored evaluation score. Requires `score:`.',
    block:     '**block** — Blocks the operation.',
    allow:     '**allow** — Explicitly allows the operation.',
    noop:      '**noop** — No operation. Useful as a placeholder.',
};

class AxiomHoverProvider implements vscode.HoverProvider {
    provideHover(
        document: vscode.TextDocument,
        position: vscode.Position,
    ): vscode.Hover | null {
        const wordRange = document.getWordRangeAtPosition(position, /[a-z_]+/);
        if (!wordRange) return null;
        const word = document.getText(wordRange);

        if (OPERATOR_DOCS[word]) {
            return new vscode.Hover(new vscode.MarkdownString(OPERATOR_DOCS[word]), wordRange);
        }
        if (ACTION_DOCS[word]) {
            return new vscode.Hover(new vscode.MarkdownString(ACTION_DOCS[word]), wordRange);
        }

        const conditionDocs: Record<string, string> = {
            all:  '**all** — Logical AND. All child conditions must be true.',
            any:  '**any** — Logical OR. At least one child condition must be true.',
            none: '**none** — Logical NOR. No child conditions may be true.',
            not:  '**not** — Logical NOT. Negates a single child condition.',
        };
        if (conditionDocs[word]) {
            return new vscode.Hover(new vscode.MarkdownString(conditionDocs[word]), wordRange);
        }

        return null;
    }
}

// ---------------------------------------------------------------------------
// Completion Provider
// ---------------------------------------------------------------------------

class AxiomCompletionProvider implements vscode.CompletionItemProvider {
    provideCompletionItems(
        document: vscode.TextDocument,
        position: vscode.Position,
    ): vscode.CompletionItem[] {
        const line = document.lineAt(position).text.slice(0, position.character);

        // Operator completions
        if (/^\s*(op|operator)\s*:\s*$/.test(line)) {
            return Object.keys(OPERATOR_DOCS).map((op) => {
                const item = new vscode.CompletionItem(op, vscode.CompletionItemKind.EnumMember);
                item.detail = 'ARS operator';
                item.documentation = new vscode.MarkdownString(OPERATOR_DOCS[op]);
                return item;
            });
        }

        // Action type completions
        if (/^\s*-?\s*type\s*:\s*$/.test(line) || /^\s*type\s*:\s*$/.test(line)) {
            return Object.keys(ACTION_DOCS).map((t) => {
                const item = new vscode.CompletionItem(t, vscode.CompletionItemKind.Function);
                item.detail = 'ARS action type';
                item.documentation = new vscode.MarkdownString(ACTION_DOCS[t]);
                return item;
            });
        }

        // Strategy completions
        if (/^\s*strategy\s*:\s*$/.test(line)) {
            return ['first_match', 'all_match', 'scored'].map((s) => {
                const item = new vscode.CompletionItem(s, vscode.CompletionItemKind.EnumMember);
                item.detail = 'Evaluation strategy';
                return item;
            });
        }

        return [];
    }
}

// ---------------------------------------------------------------------------
// Preview Rule JSON
// ---------------------------------------------------------------------------

async function previewRuleJson(context: vscode.ExtensionContext): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    if (!editor || !isAxiomDocument(editor.document)) {
        vscode.window.showWarningMessage('Axiom: No active ARS file.');
        return;
    }

    const text = editor.document.getText();
    let parsed: unknown;
    try {
        parsed = yaml.load(text);
    } catch (e) {
        vscode.window.showErrorMessage(`Axiom: Parse error — ${e}`);
        return;
    }

    const jsonText = JSON.stringify(parsed, null, 2);
    const doc = await vscode.workspace.openTextDocument({
        content: jsonText,
        language: 'json',
    });
    await vscode.window.showTextDocument(doc, vscode.ViewColumn.Beside);
}

// ---------------------------------------------------------------------------
// Open Visual Builder
// ---------------------------------------------------------------------------

function openVisualBuilder(_context: vscode.ExtensionContext): void {
    const cfg = vscode.workspace.getConfiguration('axiom');
    const serverUrl = cfg.get<string>('serverUrl', 'http://localhost:8080');
    const builderUrl = `${serverUrl.replace(/\/$/, '')}/rules/new`;

    vscode.env.openExternal(vscode.Uri.parse(builderUrl));
}

// ---------------------------------------------------------------------------
// Utility helpers
// ---------------------------------------------------------------------------

function findLineOf(text: string, needle: string, defaultLine: number): number {
    const lines = text.split('\n');
    const idx = lines.findIndex((l) => l.includes(needle));
    return idx >= 0 ? idx : defaultLine;
}

function makeDiag(
    doc: vscode.TextDocument,
    lineNum: number,
    message: string,
    severity: vscode.DiagnosticSeverity,
): vscode.Diagnostic {
    const line = Math.min(lineNum, doc.lineCount - 1);
    const range = doc.lineAt(line).range;
    const d = new vscode.Diagnostic(range, message, severity);
    d.source = 'axiom';
    return d;
}
