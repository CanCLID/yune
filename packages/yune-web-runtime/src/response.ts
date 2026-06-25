import type { YuneWebBindings } from "./module.js";

export interface YuneWebCandidate {
  text: string;
  comment: string;
  source?: string;
  quality?: number;
  preedit?: string;
  ai_confidence?: number;
}

export interface YuneWebInspectorSegment {
  start: number;
  end: number;
  tag: string;
  source: string;
}

export interface YuneWebFilterAuditRecord {
  name: string;
  before_count: number;
  after_count: number;
}

export interface YuneWebSpellingAlgebraDebug {
  translator: string;
  input: string;
  lookup_code: string | null;
  formulas: string[];
  expanded_codes: string[];
}

export interface YuneWebPredictionCandidateDebug {
  index: number;
  text: string;
  source: string;
  quality: number;
  threshold: number | null;
  above_threshold: boolean | null;
}

export interface YuneWebInspectorDebug {
  segment_tags: string[];
  segments: YuneWebInspectorSegment[];
  filter_pipeline: string[];
  filter_audit: YuneWebFilterAuditRecord[];
  spelling_algebra: YuneWebSpellingAlgebraDebug[];
  prediction: {
    weight_threshold: number | null;
    candidates: YuneWebPredictionCandidateDebug[];
  };
  ai_staging: {
    state: string;
    for_input: string | null;
  };
}

export interface YuneWebContext {
  input: string;
  preedit: string;
  caret: number;
  highlighted: number;
  page_size: number;
  page_no: number;
  is_last_page: boolean;
  select_keys: string | null;
  select_labels: string[];
  candidates: YuneWebCandidate[];
  debug?: YuneWebInspectorDebug;
}

export interface YuneWebStatus {
  schema_id: string;
  schema_name: string;
  is_disabled: boolean;
  is_composing: boolean;
  is_ascii_mode: boolean;
  is_full_shape: boolean;
  is_simplified: boolean;
  is_traditional: boolean;
  is_ascii_punct: boolean;
}

export interface YuneWebResponse {
  handled: boolean;
  commits: string[];
  context: YuneWebContext | null;
  status: YuneWebStatus | null;
  error?: string;
}

export class YuneWebResponseError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "YuneWebResponseError";
  }
}

export function readYuneWebResponse(
  responsePtr: number,
  bindings: YuneWebBindings,
): YuneWebResponse {
  if (responsePtr === 0) {
    throw new YuneWebResponseError("YuneWeb adapter returned null response");
  }

  try {
    const jsonPtr = bindings.responseJson(responsePtr);
    if (jsonPtr === 0) {
      throw new YuneWebResponseError("YuneWeb adapter returned null response JSON");
    }

    const text = bindings.module.UTF8ToString(jsonPtr);
    const parsed = parseResponseJson(text);
    const response = parseYuneWebResponse(parsed);
    response.handled = bindings.responseHandled(responsePtr) !== 0;
    return response;
  } finally {
    bindings.freeResponse(responsePtr);
  }
}

function parseResponseJson(text: string): unknown {
  try {
    return JSON.parse(text) as unknown;
  } catch {
    throw new YuneWebResponseError("YuneWeb adapter returned malformed response JSON");
  }
}

function parseYuneWebResponse(value: unknown): YuneWebResponse {
  const object = expectRecord(value, "YuneWeb response must be an object");
  if (typeof object.handled !== "boolean") {
    throw new YuneWebResponseError("YuneWeb response handled field must be boolean");
  }
  if (!Array.isArray(object.commits) || !object.commits.every((commit) => typeof commit === "string")) {
    throw new YuneWebResponseError("YuneWeb response commits field must be a string array");
  }

  const response: YuneWebResponse = {
    handled: object.handled,
    commits: object.commits,
    context: parseNullable(object.context, parseYuneWebContext, "YuneWeb response context field is required"),
    status: parseNullable(object.status, parseYuneWebStatus, "YuneWeb response status field is required"),
  };

  if (object.error !== undefined) {
    if (typeof object.error !== "string") {
      throw new YuneWebResponseError("YuneWeb response error field must be a string");
    }
    response.error = object.error;
  }

  return response;
}

function parseYuneWebContext(value: unknown): YuneWebContext {
  const object = expectRecord(value, "YuneWeb context must be an object");
  const context: YuneWebContext = {
    input: expectString(object.input, "YuneWeb context input must be a string"),
    preedit: expectString(object.preedit, "YuneWeb context preedit must be a string"),
    caret: expectNumber(object.caret, "YuneWeb context caret must be a number"),
    highlighted: expectNumber(object.highlighted, "YuneWeb context highlighted must be a number"),
    page_size: expectNumber(object.page_size, "YuneWeb context page_size must be a number"),
    page_no: expectNumber(object.page_no, "YuneWeb context page_no must be a number"),
    is_last_page: expectBoolean(object.is_last_page, "YuneWeb context is_last_page must be boolean"),
    select_keys: parseNullable(object.select_keys, (item) => expectString(item, "YuneWeb context select_keys must be a string"), "YuneWeb context select_keys field is required"),
    select_labels: expectStringArray(object.select_labels, "YuneWeb context select_labels must be a string array"),
    candidates: parseCandidates(object.candidates),
  };
  const debug = parseOptional(object.debug, parseInspectorDebug);
  if (debug !== undefined) {
    context.debug = debug;
  }
  return context;
}

function parseYuneWebStatus(value: unknown): YuneWebStatus {
  const object = expectRecord(value, "YuneWeb status must be an object");
  return {
    schema_id: expectString(object.schema_id, "YuneWeb status schema_id must be a string"),
    schema_name: expectString(object.schema_name, "YuneWeb status schema_name must be a string"),
    is_disabled: expectBoolean(object.is_disabled, "YuneWeb status is_disabled must be boolean"),
    is_composing: expectBoolean(object.is_composing, "YuneWeb status is_composing must be boolean"),
    is_ascii_mode: expectBoolean(object.is_ascii_mode, "YuneWeb status is_ascii_mode must be boolean"),
    is_full_shape: expectBoolean(object.is_full_shape, "YuneWeb status is_full_shape must be boolean"),
    is_simplified: expectBoolean(object.is_simplified, "YuneWeb status is_simplified must be boolean"),
    is_traditional: expectBoolean(object.is_traditional, "YuneWeb status is_traditional must be boolean"),
    is_ascii_punct: expectBoolean(object.is_ascii_punct, "YuneWeb status is_ascii_punct must be boolean"),
  };
}

function parseCandidates(value: unknown): YuneWebCandidate[] {
  if (!Array.isArray(value)) {
    throw new YuneWebResponseError("YuneWeb context candidates must be an array");
  }
  return value.map((candidate) => {
    const object = expectRecord(candidate, "YuneWeb candidate must be an object");
    const parsed: YuneWebCandidate = {
      text: expectString(object.text, "YuneWeb candidate text must be a string"),
      comment: expectString(object.comment, "YuneWeb candidate comment must be a string"),
    };
    if (object.source !== undefined && object.source !== null) {
      parsed.source = expectString(object.source, "YuneWeb candidate source must be a string");
    }
    if (object.quality !== undefined && object.quality !== null) {
      parsed.quality = expectNumber(object.quality, "YuneWeb candidate quality must be a number");
    }
    if (object.preedit !== undefined && object.preedit !== null) {
      parsed.preedit = expectString(object.preedit, "YuneWeb candidate preedit must be a string");
    }
    if (object.ai_confidence !== undefined && object.ai_confidence !== null) {
      parsed.ai_confidence = expectNumber(object.ai_confidence, "YuneWeb candidate ai_confidence must be a number");
    }
    return parsed;
  });
}

function parseInspectorDebug(value: unknown): YuneWebInspectorDebug {
  const object = expectRecord(value, "YuneWeb inspector debug must be an object");
  const prediction = expectRecord(object.prediction, "YuneWeb inspector prediction must be an object");
  const aiStaging = expectRecord(object.ai_staging, "YuneWeb inspector AI staging must be an object");
  return {
    segment_tags: expectStringArray(object.segment_tags, "YuneWeb inspector segment_tags must be a string array"),
    segments: parseArray(object.segments, parseInspectorSegment, "YuneWeb inspector segments must be an array"),
    filter_pipeline: expectStringArray(object.filter_pipeline, "YuneWeb inspector filter_pipeline must be a string array"),
    filter_audit: parseArray(object.filter_audit, parseFilterAuditRecord, "YuneWeb inspector filter_audit must be an array"),
    spelling_algebra: parseArray(object.spelling_algebra, parseSpellingAlgebraDebug, "YuneWeb inspector spelling_algebra must be an array"),
    prediction: {
      weight_threshold: parseNullable(prediction.weight_threshold, (item) => expectNumber(item, "YuneWeb inspector prediction weight_threshold must be a number"), "YuneWeb inspector prediction weight_threshold field is required"),
      candidates: parseArray(prediction.candidates, parsePredictionCandidateDebug, "YuneWeb inspector prediction candidates must be an array"),
    },
    ai_staging: {
      state: expectString(aiStaging.state, "YuneWeb inspector AI staging state must be a string"),
      for_input: parseNullable(aiStaging.for_input, (item) => expectString(item, "YuneWeb inspector AI staging for_input must be a string"), "YuneWeb inspector AI staging for_input field is required"),
    },
  };
}

function parseInspectorSegment(value: unknown): YuneWebInspectorSegment {
  const object = expectRecord(value, "YuneWeb inspector segment must be an object");
  return {
    start: expectNumber(object.start, "YuneWeb inspector segment start must be a number"),
    end: expectNumber(object.end, "YuneWeb inspector segment end must be a number"),
    tag: expectString(object.tag, "YuneWeb inspector segment tag must be a string"),
    source: expectString(object.source, "YuneWeb inspector segment source must be a string"),
  };
}

function parseFilterAuditRecord(value: unknown): YuneWebFilterAuditRecord {
  const object = expectRecord(value, "YuneWeb inspector filter audit record must be an object");
  return {
    name: expectString(object.name, "YuneWeb inspector filter audit name must be a string"),
    before_count: expectNumber(object.before_count, "YuneWeb inspector filter audit before_count must be a number"),
    after_count: expectNumber(object.after_count, "YuneWeb inspector filter audit after_count must be a number"),
  };
}

function parseSpellingAlgebraDebug(value: unknown): YuneWebSpellingAlgebraDebug {
  const object = expectRecord(value, "YuneWeb inspector spelling algebra must be an object");
  return {
    translator: expectString(object.translator, "YuneWeb inspector spelling algebra translator must be a string"),
    input: expectString(object.input, "YuneWeb inspector spelling algebra input must be a string"),
    lookup_code: parseNullable(object.lookup_code, (item) => expectString(item, "YuneWeb inspector spelling algebra lookup_code must be a string"), "YuneWeb inspector spelling algebra lookup_code field is required"),
    formulas: expectStringArray(object.formulas, "YuneWeb inspector spelling algebra formulas must be a string array"),
    expanded_codes: expectStringArray(object.expanded_codes, "YuneWeb inspector spelling algebra expanded_codes must be a string array"),
  };
}

function parsePredictionCandidateDebug(value: unknown): YuneWebPredictionCandidateDebug {
  const object = expectRecord(value, "YuneWeb inspector prediction candidate must be an object");
  return {
    index: expectNumber(object.index, "YuneWeb inspector prediction candidate index must be a number"),
    text: expectString(object.text, "YuneWeb inspector prediction candidate text must be a string"),
    source: expectString(object.source, "YuneWeb inspector prediction candidate source must be a string"),
    quality: expectNumber(object.quality, "YuneWeb inspector prediction candidate quality must be a number"),
    threshold: parseNullable(object.threshold, (item) => expectNumber(item, "YuneWeb inspector prediction candidate threshold must be a number"), "YuneWeb inspector prediction candidate threshold field is required"),
    above_threshold: parseNullable(object.above_threshold, (item) => expectBoolean(item, "YuneWeb inspector prediction candidate above_threshold must be boolean"), "YuneWeb inspector prediction candidate above_threshold field is required"),
  };
}

function parseOptional<T>(
  value: unknown,
  parser: (value: unknown) => T,
): T | undefined {
  if (value === undefined || value === null) {
    return undefined;
  }
  return parser(value);
}

function parseNullable<T>(
  value: unknown,
  parser: (value: unknown) => T,
  missingMessage: string,
): T | null {
  if (value === undefined) {
    throw new YuneWebResponseError(missingMessage);
  }
  if (value === null) {
    return null;
  }
  return parser(value);
}

function parseArray<T>(
  value: unknown,
  parser: (value: unknown) => T,
  message: string,
): T[] {
  if (!Array.isArray(value)) {
    throw new YuneWebResponseError(message);
  }
  return value.map(parser);
}

function expectRecord(value: unknown, message: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new YuneWebResponseError(message);
  }
  return value as Record<string, unknown>;
}

function expectString(value: unknown, message: string): string {
  if (typeof value !== "string") {
    throw new YuneWebResponseError(message);
  }
  return value;
}

function expectStringArray(value: unknown, message: string): string[] {
  if (!Array.isArray(value) || !value.every((item) => typeof item === "string")) {
    throw new YuneWebResponseError(message);
  }
  return value;
}

function expectNumber(value: unknown, message: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new YuneWebResponseError(message);
  }
  return value;
}

function expectBoolean(value: unknown, message: string): boolean {
  if (typeof value !== "boolean") {
    throw new YuneWebResponseError(message);
  }
  return value;
}
