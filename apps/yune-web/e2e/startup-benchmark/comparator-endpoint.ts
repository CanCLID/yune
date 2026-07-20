export const comparatorEndpointContractVersion = "web06-comparator-endpoint-v1" as const;
export const comparatorIdentityContractVersion = "web06-peer-data-v1" as const;
export const comparatorPeerPageSize = 6 as const;
export const comparatorPeerCadenceMs = 60 as const;
export const comparatorBindingRoundCount = 5 as const;
export const comparatorPinnedMyRimeCommit = "c73ea172d28f07031ba87a1d71c4d2e1c8ba82a3" as const;

export type ComparatorApp = "yune-web" | "my-rime";
export type PackageAlignment = "PROVED" | "DATA_CONFOUNDED";

export interface ComparatorSelectorManifest {
  id: string;
  editable: string;
  candidateRoot: string;
  candidateSurface: string;
  candidateRows: string;
  composition: string;
  candidateLabel: string;
  candidateText: string;
  candidateComment: string;
  highlighted: string;
  pageButtons: string;
  status: string;
}

export const comparatorSelectorManifest: Record<ComparatorApp, ComparatorSelectorManifest> = {
  "yune-web": {
    id: "yune-web-public-dom-v1",
    editable: "textarea.yd-input-area",
    candidateRoot: ".candidate-panel",
    candidateSurface: ".candidate-panel .candidates",
    candidateRows: ".candidate-row",
    composition: ".candidate-preedit",
    candidateLabel: ".candidate-index",
    candidateText: ".candidate-text",
    candidateComment: ".candidate-note",
    highlighted: ".candidate-row.highlighted",
    pageButtons: ".page-nav",
    status: "[data-yune-status]",
  },
  "my-rime": {
    id: "my-rime-c73ea17-public-dom-v1",
    editable: "#container textarea",
    candidateRoot: ".n-popover",
    candidateSurface: ".n-popover .n-menu",
    candidateRows: ".n-menu-item",
    composition: ".n-popover",
    candidateLabel: "",
    candidateText: "",
    candidateComment: "",
    highlighted: ".n-menu-item-content--selected, [aria-selected='true']",
    pageButtons: "button",
    status: ".n-popover",
  },
};

export interface ComparatorCandidateTuple {
  label: string;
  text: string;
  comment: string;
}

export interface ComparatorDomTuple {
  contractVersion: typeof comparatorEndpointContractVersion;
  selectorManifestId: string;
  revision: number;
  observedAt: number;
  composition: string;
  candidates: ComparatorCandidateTuple[];
  candidateSurfaceCount: number;
  page: {
    index: number | null;
    buttonCount: number;
    previousDisabled: boolean | null;
    nextDisabled: boolean | null;
  };
  highlightedIndex: number;
  caret: {
    selectorCount: number;
    value: string;
    selectionStart: number | null;
    selectionEnd: number | null;
    selectionDirection: string | null;
    active: boolean;
    visible: boolean;
    disabled: boolean;
  };
  status: {
    schemaId: string;
    composing: boolean;
    surfaceVisible: boolean;
    digest: string;
  };
  digest: string;
}

export interface ComparatorEventBoundary {
  ordinal: number;
  type: "keydown";
  key: string;
  code: string;
  timeStamp: number;
  revisionBeforeEvent: number;
}

export interface ComparatorStableObservation {
  event: ComparatorEventBoundary;
  initial: ComparatorDomTuple;
  firstRaf: ComparatorDomTuple;
  secondRaf: ComparatorDomTuple;
  yuneDiagnostic?: {
    index: number;
    input: string;
    renderedInput: string;
    renderRevision: number;
    candidateCount: number;
    totalCandidateCount: number;
    firstCandidateText: string;
  };
}

export interface ComparatorPageSizeSetup {
  contractVersion: "web06-page-size-setup-v1";
  requiredRows: 6;
  initial: {
    uiValue: string;
    localStorageValue: string | null;
    persistedConfigValue: string | null;
    deployStatus: string | null;
    persistenceDiagnosticCount: number;
  };
  actions: Array<{
    ordinal: number;
    fromUiValue: string;
    targetUiValue: string;
    interaction: {
      kind: "keyboard";
      key: "ArrowRight" | "ArrowLeft";
      control: "preferences-page-size-range";
    };
    deployStatus: string | null;
    loadingComplete: boolean;
    localStorageValue: string | null;
    persistedConfigValue: string | null;
    persistenceDiagnosticIndex: number;
    engineProbe: {
      input: "ni";
      candidateRows: number;
      candidates: string[];
      pageIndex: number | null;
      buttonCount: number;
      previousDisabled: boolean | null;
      nextDisabled: boolean | null;
      resetKey: "Escape";
      resetEmpty: boolean;
    };
  }>;
  final: {
    uiValue: string;
    localStorageValue: string | null;
    persistedConfigValue: string | null;
    deployStatus: string | null;
    loadingComplete: boolean;
  };
  measurementPage: {
    initial: {
      uiValue: string;
      localStorageValue: string | null;
      persistedConfigValue: string | null;
      deployStatus: string | null;
      persistenceDiagnosticCount: number;
    };
    actions: Array<{
      ordinal: number;
      fromUiValue: string;
      targetUiValue: string;
      interaction: {
        kind: "keyboard";
        key: "ArrowRight" | "ArrowLeft";
        control: "preferences-page-size-range";
      };
      deployStatus: string | null;
      loadingComplete: boolean;
      localStorageValue: string | null;
      persistedConfigValue: string | null;
      persistenceDiagnosticIndex: number;
    }>;
    final: {
      uiValue: string;
      localStorageValue: string | null;
      persistedConfigValue: string | null;
      deployStatus: string | null;
      loadingComplete: boolean;
    };
  };
  engineProof: {
    candidateRows: number;
    pageIndex: number | null;
    buttonCount: number;
    previousDisabled: boolean | null;
    nextDisabled: boolean | null;
  };
}

export interface ComparatorEndpointEvidence {
  inputEvents: ComparatorEventBoundary[];
  candidate: ComparatorStableObservation;
  commit: ComparatorStableObservation;
}

export const comparatorPeerLogicalInputIds = [
  "resolved-schema-includes-patches",
  "dictionary-and-imports",
  "essay",
  "grammar-model",
  "speller-algebra",
  "filters-and-options",
  "page-size-and-comments",
  "fresh-empty-userdb",
] as const;

export interface ComparatorReproducibleSide {
  repositoryCommit: string;
  upstreamPinnedCommit: string;
  artifactSourceCommit: string;
  artifactSourceTree: string;
  sourceTreeState: "clean";
  artifactSha256: string;
  generatedManifestSha256: string;
  completeArtifactManifestSha256: string;
  buildCommand: string;
  packageManager: {
    name: "npm" | "pnpm";
    version: string;
    lockSha256: string;
    integrityManifestSha256: string;
  };
  toolchain: {
    nodeVersion: string;
    emscriptenVersion: string;
    emscriptenCommit: string;
    compilerVersion: string;
  };
  resolvedRecipes: Array<{
    id: string;
    repository: string;
    commit: string;
    logicalBytesSha256: string;
  }>;
  compiledHashes: {
    table: string;
    prism: string;
    reverse: string;
    "data-model": string;
    runtime: string;
  };
}

export interface ComparatorIdentityManifest {
  version: typeof comparatorIdentityContractVersion;
  yune: ComparatorReproducibleSide;
  peer: ComparatorReproducibleSide;
  logicalInputs: Array<{
    id: typeof comparatorPeerLogicalInputIds[number];
    yuneSha256: string;
    peerSha256: string;
    explicitNone?: boolean;
  }>;
  effectiveConfiguration: {
    yuneSha256: string;
    peerSha256: string;
  };
  freshEmptyUserdb: boolean;
  sameEndpointObserver: boolean;
}

export interface AlignmentVerdict {
  packageAlignment: PackageAlignment;
  reasons: string[];
}

export function comparatorDomTupleDigest(tuple: ComparatorDomTuple): string {
  return JSON.stringify({
    contractVersion: tuple.contractVersion,
    selectorManifestId: tuple.selectorManifestId,
    composition: tuple.composition,
    candidates: tuple.candidates,
    candidateSurfaceCount: tuple.candidateSurfaceCount,
    page: tuple.page,
    highlightedIndex: tuple.highlightedIndex,
    caret: tuple.caret,
    status: tuple.status,
  });
}

export function parseComparatorIdentityManifest(text: string): ComparatorIdentityManifest {
  const value = JSON.parse(text) as unknown;
  const errors: string[] = [];
  if (!isRecord(value)) {
    throw new Error("Comparator identity manifest must be an object");
  }
  validateIdentityManifest(value, errors);
  if (errors.length > 0) {
    throw new Error("Invalid comparator identity manifest: " + errors.join("; "));
  }
  return value as unknown as ComparatorIdentityManifest;
}

export function evaluatePackageAlignment(
  manifest: ComparatorIdentityManifest | undefined,
): AlignmentVerdict {
  const reasons: string[] = [];
  if (!manifest) {
    reasons.push("missing-peer-data-manifest");
    return { packageAlignment: "DATA_CONFOUNDED", reasons };
  }
  if (manifest.version !== comparatorIdentityContractVersion) reasons.push("manifest-version");
  validateReproducibleSide("yune", manifest.yune, reasons);
  validateReproducibleSide("peer", manifest.peer, reasons);
  const logicalInputs = Array.isArray(manifest.logicalInputs) ? manifest.logicalInputs : [];
  const logical = new Map(logicalInputs.map(item => [item.id, item]));
  for (const id of comparatorPeerLogicalInputIds) {
    const item = logical.get(id);
    if (!item) {
      reasons.push("logical-input-missing:" + id);
      continue;
    }
    if (id === "grammar-model" && item.explicitNone === true) {
      if (item.yuneSha256 !== "none" || item.peerSha256 !== "none") {
        reasons.push("grammar-explicit-none-mismatch");
      }
    } else if (!isSha256(item.yuneSha256) || !isSha256(item.peerSha256)) {
      reasons.push("logical-input-hash:" + id);
    }
    if (item.yuneSha256 !== item.peerSha256) {
      reasons.push("logical-input-different:" + id);
    }
  }
  if (logicalInputs.length !== comparatorPeerLogicalInputIds.length
      || logical.size !== comparatorPeerLogicalInputIds.length) {
    reasons.push("logical-input-extra-or-duplicate");
  }
  if (!isSha256(manifest.effectiveConfiguration?.yuneSha256)
      || manifest.effectiveConfiguration.yuneSha256 !== manifest.effectiveConfiguration.peerSha256) {
    reasons.push("effective-configuration-different");
  }
  if (manifest.freshEmptyUserdb !== true) reasons.push("fresh-empty-userdb-not-proved");
  if (manifest.sameEndpointObserver !== true) reasons.push("same-endpoint-observer-not-proved");
  return {
    packageAlignment: reasons.length === 0 ? "PROVED" : "DATA_CONFOUNDED",
    reasons,
  };
}

export function validateCandidateObservation(
  observation: ComparatorStableObservation | undefined,
  expectedInput: string,
): string[] {
  const reasons = validateStableObservation(observation);
  if (!observation) {
    return reasons;
  }
  const tuple = observation.secondRaf;
  if (observation.event.key !== expectedInput.at(-1)) {
    reasons.push("candidate-final-event-key-mismatch");
  }
  if (observation.event.code !== expectedCode(expectedInput.at(-1) ?? "")) {
    reasons.push("candidate-final-event-code-mismatch");
  }
  if (tuple.composition !== expectedInput) {
    reasons.push("candidate-composition-is-not-complete-input");
  }
  if (tuple.candidateSurfaceCount !== 1 || tuple.candidates.length === 0) {
    reasons.push("candidate-surface-is-not-one-complete-visible-collection");
  }
  if (tuple.candidates.some(candidate => candidate.text === "")) {
    reasons.push("candidate-collection-contains-empty-visible-text");
  }
  if (tuple.candidates.some((candidate, index) =>
    candidate.label.replace(/[.\s]/g, "") !== String(index + 1)
  )) {
    reasons.push("candidate-collection-label-order-is-not-the-frozen-default");
  }
  if (tuple.candidates.length !== comparatorPeerPageSize) {
    reasons.push("candidate-page-size-is-not-six");
  }
  if (tuple.page.index !== 0
      || tuple.page.buttonCount !== 2
      || tuple.page.previousDisabled !== true
      || tuple.page.nextDisabled !== false) {
    reasons.push("candidate-page-evidence-incomplete");
  }
  if (tuple.highlightedIndex < 0 || tuple.highlightedIndex >= tuple.candidates.length) {
    reasons.push("candidate-highlight-evidence-incomplete");
  }
  if (tuple.caret.selectorCount !== 1 || !tuple.caret.active || !tuple.caret.visible || tuple.caret.disabled || tuple.caret.selectionStart === null || tuple.caret.selectionEnd === null) {
    reasons.push("candidate-caret-evidence-incomplete");
  }
  if (!tuple.status.schemaId || !tuple.status.composing || !tuple.status.surfaceVisible) {
    reasons.push("candidate-status-evidence-incomplete");
  }
  if (tuple.selectorManifestId.startsWith("yune-web-")) {
    const diagnostic = observation.yuneDiagnostic;
    if (!diagnostic
        || diagnostic.input !== expectedInput
        || diagnostic.renderedInput !== expectedInput
        || diagnostic.firstCandidateText !== tuple.candidates[0]?.text
        || diagnostic.candidateCount !== tuple.candidates.length
        || diagnostic.renderRevision <= 0) {
      reasons.push("yune-final-input-diagnostic-is-not-coherent-with-dom");
    }
  }
  return reasons;
}

export function validateCommitObservation(
  observation: ComparatorStableObservation | undefined,
  expectedCommittedValue: string,
): string[] {
  const reasons = validateStableObservation(observation);
  if (!observation) {
    return reasons;
  }
  const tuple = observation.secondRaf;
  if (observation.event.key !== " " || observation.event.code !== "Space") {
    reasons.push("commit-event-is-not-space-key-code");
  }
  if (tuple.caret.value !== expectedCommittedValue) {
    reasons.push("committed-visible-value-mismatch");
  }
  if (tuple.caret.selectionStart !== expectedCommittedValue.length || tuple.caret.selectionEnd !== expectedCommittedValue.length) {
    reasons.push("committed-caret-mismatch");
  }
  if (tuple.composition !== "" || tuple.candidateSurfaceCount !== 0 || tuple.candidates.length !== 0) {
    reasons.push("commit-left-visible-composition-or-candidates");
  }
  if (tuple.caret.selectorCount !== 1 || !tuple.caret.active || !tuple.caret.visible || tuple.caret.disabled || tuple.status.composing || tuple.status.surfaceVisible) {
    reasons.push("commit-visible-surface-status-incomplete");
  }
  if (!tuple.status.schemaId) {
    reasons.push("commit-schema-status-missing");
  }
  return reasons;
}

export function validateEndpointEvidence(
  endpoint: ComparatorEndpointEvidence | undefined,
  expectedInput: string,
  expectedCommittedValue: string,
  expectedApp?: ComparatorApp,
): string[] {
  if (!endpoint) {
    return ["missing-endpoint-evidence"];
  }
  return [
    ...validateInputEventSchedule(endpoint, expectedInput),
    ...validateCandidateCommitSequence(endpoint),
    ...validateCandidateObservation(endpoint.candidate, expectedInput),
    ...validateCommitObservation(endpoint.commit, expectedCommittedValue),
    ...(expectedApp ? validateSelectorManifest(endpoint, expectedApp) : []),
  ];
}

function validateSelectorManifest(endpoint: ComparatorEndpointEvidence, app: ComparatorApp): string[] {
  const expected = comparatorSelectorManifest[app].id;
  const tuples = [
    endpoint.candidate.initial,
    endpoint.candidate.firstRaf,
    endpoint.candidate.secondRaf,
    endpoint.commit.initial,
    endpoint.commit.firstRaf,
    endpoint.commit.secondRaf,
  ];
  return tuples.every(tuple => tuple.selectorManifestId === expected)
    ? []
    : ["endpoint-frozen-selector-manifest-not-proved"];
}

function validateInputEventSchedule(
  endpoint: ComparatorEndpointEvidence,
  expectedInput: string,
): string[] {
  const reasons: string[] = [];
  if (endpoint.inputEvents.some(event =>
    event.type !== "keydown"
    || !Number.isInteger(event.ordinal)
    || event.ordinal <= 0
    || !Number.isInteger(event.revisionBeforeEvent)
    || event.revisionBeforeEvent < 0
    || !Number.isFinite(event.timeStamp)
    || event.timeStamp < 0
  )) {
    reasons.push("input-event-boundary-invalid");
  }
  if (endpoint.inputEvents.length !== expectedInput.length
      || endpoint.inputEvents.map(event => event.key).join("") !== expectedInput) {
    reasons.push("input-event-sequence-does-not-equal-complete-input");
  }
  if (endpoint.inputEvents.some((event, index) => event.code !== expectedCode(expectedInput[index] ?? ""))) {
    reasons.push("input-event-code-sequence-does-not-equal-complete-input");
  }
  const finalInputEvent = endpoint.inputEvents.at(-1);
  if (!finalInputEvent
      || finalInputEvent.ordinal !== endpoint.candidate.event.ordinal
      || finalInputEvent.timeStamp !== endpoint.candidate.event.timeStamp
      || finalInputEvent.revisionBeforeEvent !== endpoint.candidate.event.revisionBeforeEvent
      || finalInputEvent.key !== endpoint.candidate.event.key
      || finalInputEvent.code !== endpoint.candidate.event.code) {
    reasons.push("candidate-boundary-is-not-the-final-input-event");
  }
  for (let index = 1; index < endpoint.inputEvents.length; index += 1) {
    const previous = endpoint.inputEvents[index - 1];
    const current = endpoint.inputEvents[index];
    if (!previous || !current || current.ordinal !== previous.ordinal + 1) {
      reasons.push("input-event-ordinals-are-not-contiguous");
      break;
    }
    const gap = current.timeStamp - previous.timeStamp;
    if (!Number.isFinite(gap) || gap < 48 || gap > 75) {
      reasons.push("input-event-gap-is-outside-frozen-60ms-cadence");
      break;
    }
  }
  return reasons;
}

function validateCandidateCommitSequence(endpoint: ComparatorEndpointEvidence): string[] {
  const reasons: string[] = [];
  if (endpoint.commit.event.ordinal !== endpoint.candidate.event.ordinal + 1) {
    reasons.push("commit-event-does-not-immediately-follow-final-input-event");
  }
  if (endpoint.commit.event.timeStamp < endpoint.candidate.secondRaf.observedAt) {
    reasons.push("commit-event-precedes-coherent-candidate-endpoint");
  }
  if (endpoint.commit.event.revisionBeforeEvent < endpoint.candidate.secondRaf.revision) {
    reasons.push("commit-event-revision-precedes-candidate-endpoint");
  }
  return reasons;
}

function expectedCode(key: string): string {
  return /^[a-z]$/i.test(key) ? "Key" + key.toUpperCase() : key;
}

function validateStableObservation(observation: ComparatorStableObservation | undefined): string[] {
  if (!observation) {
    return ["missing-stable-observation"];
  }
  const reasons: string[] = [];
  const tuples = [observation.initial, observation.firstRaf, observation.secondRaf];
  if (!Number.isInteger(observation.event.ordinal)
      || observation.event.ordinal <= 0
      || !Number.isInteger(observation.event.revisionBeforeEvent)
      || observation.event.revisionBeforeEvent < 0
      || !Number.isFinite(observation.event.timeStamp)
      || observation.event.timeStamp < 0) {
    reasons.push("event-boundary-invalid");
  }
  if (tuples.some(tuple =>
    !Number.isInteger(tuple.revision)
    || tuple.revision < 0
    || !Number.isFinite(tuple.observedAt)
    || tuple.observedAt < 0
  )) {
    reasons.push("dom-observation-boundary-invalid");
  }
  if (tuples.some(tuple => tuple.contractVersion !== comparatorEndpointContractVersion)) {
    reasons.push("endpoint-contract-version-mismatch");
  }
  if (new Set(tuples.map(tuple => tuple.selectorManifestId)).size !== 1) {
    reasons.push("selector-manifest-changed-during-observation");
  }
  if (new Set(tuples.map(tuple => tuple.revision)).size !== 1) {
    reasons.push("dom-revision-changed-during-double-raf");
  }
  if (new Set(tuples.map(tuple => tuple.digest)).size !== 1) {
    reasons.push("dom-digest-changed-during-double-raf");
  }
  if (tuples.some(tuple => tuple.digest !== comparatorDomTupleDigest(tuple))) {
    reasons.push("dom-digest-does-not-match-atomic-tuple");
  }
  if (observation.initial.revision <= observation.event.revisionBeforeEvent) {
    reasons.push("accepted-dom-revision-is-not-after-event");
  }
  if (observation.initial.observedAt < observation.event.timeStamp) {
    reasons.push("accepted-dom-observation-precedes-event");
  }
  if (!(observation.initial.observedAt <= observation.firstRaf.observedAt
      && observation.firstRaf.observedAt <= observation.secondRaf.observedAt)) {
    reasons.push("double-raf-observation-order-invalid");
  }
  return reasons;
}

function validateIdentityManifest(value: Record<string, unknown>, errors: string[]): void {
  if (value.version !== comparatorIdentityContractVersion) {
    errors.push("manifest-version");
  }
  validateReproducibleSide("yune", value.yune, errors);
  validateReproducibleSide("peer", value.peer, errors);
  if (!Array.isArray(value.logicalInputs)) {
    errors.push("logical-inputs-missing");
  } else {
    const logicalIds = new Set<string>();
    for (const item of value.logicalInputs) {
      if (!isRecord(item) || typeof item.id !== "string") {
        errors.push("logical-input-invalid");
        continue;
      }
      if (!comparatorPeerLogicalInputIds.includes(item.id as typeof comparatorPeerLogicalInputIds[number])) {
        errors.push("logical-input-unknown:" + item.id);
      }
      if (logicalIds.has(item.id)) errors.push("logical-input-duplicate:" + item.id);
      logicalIds.add(item.id);
      if (item.id === "grammar-model" && item.explicitNone === true) {
        if (item.yuneSha256 !== "none" || item.peerSha256 !== "none") {
          errors.push("grammar-explicit-none-mismatch");
        }
      } else if (!isSha256(item.yuneSha256) || !isSha256(item.peerSha256)) {
        errors.push("logical-input-hash:" + item.id);
      }
    }
    for (const id of comparatorPeerLogicalInputIds) {
      if (!logicalIds.has(id)) errors.push("logical-input-missing:" + id);
    }
    if (value.logicalInputs.length !== comparatorPeerLogicalInputIds.length) {
      errors.push("logical-input-count");
    }
  }
  if (!isRecord(value.effectiveConfiguration)
      || !isSha256(value.effectiveConfiguration.yuneSha256)
      || !isSha256(value.effectiveConfiguration.peerSha256)) {
    errors.push("effective-configuration-hashes");
  }
  if (typeof value.freshEmptyUserdb !== "boolean") errors.push("fresh-empty-userdb-invalid");
  if (typeof value.sameEndpointObserver !== "boolean") errors.push("same-endpoint-observer-invalid");
}

function validateReproducibleSide(sideName: string, value: unknown, reasons: string[]): void {
  if (!isRecord(value)) {
    reasons.push(sideName + "-identity-missing");
    return;
  }
  if (!isCommit(value.repositoryCommit)) reasons.push(sideName + "-repository-commit");
  if (!isCommit(value.upstreamPinnedCommit)) reasons.push(sideName + "-upstream-pinned-commit");
  if (!isCommit(value.artifactSourceCommit)) reasons.push(sideName + "-artifact-source-commit");
  if (!isCommit(value.artifactSourceTree)) reasons.push(sideName + "-artifact-source-tree");
  if (value.repositoryCommit !== value.artifactSourceCommit) {
    reasons.push(sideName + "-repository-commit-is-not-artifact-source-commit");
  }
  if (value.sourceTreeState !== "clean") reasons.push(sideName + "-source-tree-state");
  if (!isSha256(value.artifactSha256)) reasons.push(sideName + "-artifact-hash");
  if (!isSha256(value.generatedManifestSha256)) reasons.push(sideName + "-generated-manifest-hash");
  if (!isSha256(value.completeArtifactManifestSha256)) {
    reasons.push(sideName + "-complete-artifact-manifest-hash");
  }
  if (typeof value.buildCommand !== "string" || value.buildCommand.trim() === "") {
    reasons.push(sideName + "-build-command");
  }
  const packageManager = value.packageManager;
  if (!isRecord(packageManager)
      || (packageManager.name !== "npm" && packageManager.name !== "pnpm")
      || typeof packageManager.version !== "string"
      || packageManager.version.trim() === ""
      || !isSha256(packageManager.lockSha256)
      || !isSha256(packageManager.integrityManifestSha256)) {
    reasons.push(sideName + "-dependency-resolution");
  }
  const toolchain = value.toolchain;
  if (!isRecord(toolchain)
      || typeof toolchain.nodeVersion !== "string"
      || toolchain.nodeVersion.trim() === ""
      || typeof toolchain.emscriptenVersion !== "string"
      || toolchain.emscriptenVersion.trim() === ""
      || !isCommit(toolchain.emscriptenCommit)
      || typeof toolchain.compilerVersion !== "string"
      || toolchain.compilerVersion.trim() === "") {
    reasons.push(sideName + "-toolchain");
  }
  const recipes = value.resolvedRecipes;
  if (!Array.isArray(recipes) || recipes.length === 0) {
    reasons.push(sideName + "-resolved-recipes");
  } else {
    const recipeIds = new Set<string>();
    for (const recipe of recipes) {
      if (!isRecord(recipe)
          || typeof recipe.id !== "string"
          || recipe.id.trim() === ""
          || typeof recipe.repository !== "string"
          || recipe.repository.trim() === ""
          || !isCommit(recipe.commit)
          || !isSha256(recipe.logicalBytesSha256)) {
        reasons.push(sideName + "-resolved-recipe:" + (isRecord(recipe) ? String(recipe.id ?? "unknown") : "unknown"));
        continue;
      }
      if (recipeIds.has(recipe.id)) reasons.push(sideName + "-resolved-recipes-duplicate");
      recipeIds.add(recipe.id);
    }
  }
  const compiled = value.compiledHashes;
  if (!isRecord(compiled)) {
    reasons.push(sideName + "-compiled-hashes");
    return;
  }
  for (const hashId of ["table", "prism", "reverse", "runtime"] as const) {
    if (!isSha256(compiled[hashId])) reasons.push(sideName + "-compiled-" + hashId);
  }
  if (!isSha256(compiled["data-model"]) && compiled["data-model"] !== "none") {
    reasons.push(sideName + "-compiled-data-model");
  }
}

function isCommit(value: unknown): value is string {
  return typeof value === "string" && /^[0-9a-f]{40}$/i.test(value);
}

function isSha256(value: unknown): value is string {
  return typeof value === "string" && /^[0-9a-f]{64}$/i.test(value);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
