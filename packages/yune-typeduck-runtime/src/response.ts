import type { TypeDuckBindings } from "./module.js";

export interface TypeDuckCandidate {
  text: string;
  comment: string;
  source?: string;
}

export interface TypeDuckContext {
  input: string;
  preedit: string;
  caret: number;
  highlighted: number;
  page_size: number;
  page_no: number;
  is_last_page: boolean;
  select_keys: string | null;
  select_labels: string[];
  candidates: TypeDuckCandidate[];
}

export interface TypeDuckStatus {
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

export interface TypeDuckResponse {
  handled: boolean;
  commits: string[];
  context: TypeDuckContext | null;
  status: TypeDuckStatus | null;
  error?: string;
}

export class TypeDuckResponseError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "TypeDuckResponseError";
  }
}

export function readTypeDuckResponse(
  responsePtr: number,
  bindings: TypeDuckBindings,
): TypeDuckResponse {
  if (responsePtr === 0) {
    throw new TypeDuckResponseError("TypeDuck adapter returned null response");
  }

  try {
    const jsonPtr = bindings.responseJson(responsePtr);
    if (jsonPtr === 0) {
      throw new TypeDuckResponseError("TypeDuck adapter returned null response JSON");
    }

    const text = bindings.module.UTF8ToString(jsonPtr);
    const parsed = parseResponseJson(text);
    const response = parseTypeDuckResponse(parsed);
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
    throw new TypeDuckResponseError("TypeDuck adapter returned malformed response JSON");
  }
}

function parseTypeDuckResponse(value: unknown): TypeDuckResponse {
  const object = expectRecord(value, "TypeDuck response must be an object");
  if (typeof object.handled !== "boolean") {
    throw new TypeDuckResponseError("TypeDuck response handled field must be boolean");
  }
  if (!Array.isArray(object.commits) || !object.commits.every((commit) => typeof commit === "string")) {
    throw new TypeDuckResponseError("TypeDuck response commits field must be a string array");
  }

  const response: TypeDuckResponse = {
    handled: object.handled,
    commits: object.commits,
    context: parseNullable(object.context, parseTypeDuckContext, "TypeDuck response context field is required"),
    status: parseNullable(object.status, parseTypeDuckStatus, "TypeDuck response status field is required"),
  };

  if (object.error !== undefined) {
    if (typeof object.error !== "string") {
      throw new TypeDuckResponseError("TypeDuck response error field must be a string");
    }
    response.error = object.error;
  }

  return response;
}

function parseTypeDuckContext(value: unknown): TypeDuckContext {
  const object = expectRecord(value, "TypeDuck context must be an object");
  return {
    input: expectString(object.input, "TypeDuck context input must be a string"),
    preedit: expectString(object.preedit, "TypeDuck context preedit must be a string"),
    caret: expectNumber(object.caret, "TypeDuck context caret must be a number"),
    highlighted: expectNumber(object.highlighted, "TypeDuck context highlighted must be a number"),
    page_size: expectNumber(object.page_size, "TypeDuck context page_size must be a number"),
    page_no: expectNumber(object.page_no, "TypeDuck context page_no must be a number"),
    is_last_page: expectBoolean(object.is_last_page, "TypeDuck context is_last_page must be boolean"),
    select_keys: parseNullable(object.select_keys, (item) => expectString(item, "TypeDuck context select_keys must be a string"), "TypeDuck context select_keys field is required"),
    select_labels: expectStringArray(object.select_labels, "TypeDuck context select_labels must be a string array"),
    candidates: parseCandidates(object.candidates),
  };
}

function parseTypeDuckStatus(value: unknown): TypeDuckStatus {
  const object = expectRecord(value, "TypeDuck status must be an object");
  return {
    schema_id: expectString(object.schema_id, "TypeDuck status schema_id must be a string"),
    schema_name: expectString(object.schema_name, "TypeDuck status schema_name must be a string"),
    is_disabled: expectBoolean(object.is_disabled, "TypeDuck status is_disabled must be boolean"),
    is_composing: expectBoolean(object.is_composing, "TypeDuck status is_composing must be boolean"),
    is_ascii_mode: expectBoolean(object.is_ascii_mode, "TypeDuck status is_ascii_mode must be boolean"),
    is_full_shape: expectBoolean(object.is_full_shape, "TypeDuck status is_full_shape must be boolean"),
    is_simplified: expectBoolean(object.is_simplified, "TypeDuck status is_simplified must be boolean"),
    is_traditional: expectBoolean(object.is_traditional, "TypeDuck status is_traditional must be boolean"),
    is_ascii_punct: expectBoolean(object.is_ascii_punct, "TypeDuck status is_ascii_punct must be boolean"),
  };
}

function parseCandidates(value: unknown): TypeDuckCandidate[] {
  if (!Array.isArray(value)) {
    throw new TypeDuckResponseError("TypeDuck context candidates must be an array");
  }
  return value.map((candidate) => {
    const object = expectRecord(candidate, "TypeDuck candidate must be an object");
    const parsed: TypeDuckCandidate = {
      text: expectString(object.text, "TypeDuck candidate text must be a string"),
      comment: expectString(object.comment, "TypeDuck candidate comment must be a string"),
    };
    if (object.source !== undefined) {
      parsed.source = expectString(object.source, "TypeDuck candidate source must be a string");
    }
    return parsed;
  });
}

function parseNullable<T>(
  value: unknown,
  parser: (value: unknown) => T,
  missingMessage: string,
): T | null {
  if (value === undefined) {
    throw new TypeDuckResponseError(missingMessage);
  }
  if (value === null) {
    return null;
  }
  return parser(value);
}

function expectRecord(value: unknown, message: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new TypeDuckResponseError(message);
  }
  return value as Record<string, unknown>;
}

function expectString(value: unknown, message: string): string {
  if (typeof value !== "string") {
    throw new TypeDuckResponseError(message);
  }
  return value;
}

function expectStringArray(value: unknown, message: string): string[] {
  if (!Array.isArray(value) || !value.every((item) => typeof item === "string")) {
    throw new TypeDuckResponseError(message);
  }
  return value;
}

function expectNumber(value: unknown, message: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new TypeDuckResponseError(message);
  }
  return value;
}

function expectBoolean(value: unknown, message: string): boolean {
  if (typeof value !== "boolean") {
    throw new TypeDuckResponseError(message);
  }
  return value;
}
