export type EmscriptenCType = "number" | "string" | "boolean" | "array" | null;

export type EmscriptenWrappedFunction = (...args: unknown[]) => unknown;

export interface EmscriptenYuneWebModule {
  cwrap(
    ident: string,
    returnType: EmscriptenCType,
    argTypes: EmscriptenCType[],
  ): EmscriptenWrappedFunction;
  UTF8ToString(ptr: number, maxBytesToRead?: number, ignoreNul?: boolean): string;
}

export const YUNE_WEB_EXPORTS = [
  "yune_web_init",
  "yune_web_process_key",
  "yune_web_select_candidate",
  "yune_web_delete_candidate",
  "yune_web_flip_page",
  "yune_web_deploy",
  "yune_web_customize",
  "yune_web_set_option",
  "yune_web_set_ai_enabled",
  "yune_web_stage_ai",
  "yune_web_cleanup",
  "yune_web_response_json",
  "yune_web_response_handled",
  "yune_web_free_response",
] as const;

export type YuneWebExport = (typeof YUNE_WEB_EXPORTS)[number];

export class YuneWebBindingError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "YuneWebBindingError";
  }
}

export interface YuneWebBindings {
  init(sharedDataDir: string, userDataDir: string, schemaId: string): number;
  processKey(statePtr: number, keycode: number, mask: number): number;
  selectCandidate(statePtr: number, index: number): number;
  deleteCandidate(statePtr: number, index: number): number;
  flipPage(statePtr: number, backward: number): number;
  customize(statePtr: number, configId: string, key: string, value: string): number;
  setOption(statePtr: number, option: string, value: number): number;
  setAiEnabled(statePtr: number, enabled: number): number;
  stageAi(statePtr: number): number;
  deploy(statePtr: number): number;
  cleanup(statePtr: number): void;
  responseJson(responsePtr: number): number;
  responseHandled(responsePtr: number): number;
  freeResponse(responsePtr: number): void;
  module: EmscriptenYuneWebModule;
}

type Signature = readonly [returnType: EmscriptenCType, argTypes: readonly EmscriptenCType[]];

const SIGNATURES: Record<YuneWebExport, Signature> = {
  yune_web_init: ["number", ["string", "string", "string"]],
  yune_web_process_key: ["number", ["number", "number", "number"]],
  yune_web_select_candidate: ["number", ["number", "number"]],
  yune_web_delete_candidate: ["number", ["number", "number"]],
  yune_web_flip_page: ["number", ["number", "number"]],
  yune_web_deploy: ["number", ["number"]],
  yune_web_customize: ["number", ["number", "string", "string", "string"]],
  yune_web_set_option: ["number", ["number", "string", "number"]],
  yune_web_set_ai_enabled: ["number", ["number", "number"]],
  yune_web_stage_ai: ["number", ["number"]],
  yune_web_cleanup: [null, ["number"]],
  yune_web_response_json: ["number", ["number"]],
  yune_web_response_handled: ["number", ["number"]],
  yune_web_free_response: [null, ["number"]],
};

const LEGACY_EXPORTS: Record<YuneWebExport, string> = {
  yune_web_init: "yune_typeduck_init",
  yune_web_process_key: "yune_typeduck_process_key",
  yune_web_select_candidate: "yune_typeduck_select_candidate",
  yune_web_delete_candidate: "yune_typeduck_delete_candidate",
  yune_web_flip_page: "yune_typeduck_flip_page",
  yune_web_deploy: "yune_typeduck_deploy",
  yune_web_customize: "yune_typeduck_customize",
  yune_web_set_option: "yune_typeduck_set_option",
  yune_web_set_ai_enabled: "yune_typeduck_set_ai_enabled",
  yune_web_stage_ai: "yune_typeduck_stage_ai",
  yune_web_cleanup: "yune_typeduck_cleanup",
  yune_web_response_json: "yune_typeduck_response_json",
  yune_web_response_handled: "yune_typeduck_response_handled",
  yune_web_free_response: "yune_typeduck_free_response",
};

export function bindYuneWebModule(module: EmscriptenYuneWebModule): YuneWebBindings {
  const wrapped = Object.fromEntries(
    YUNE_WEB_EXPORTS.map((symbol) => [symbol, bindExport(module, symbol)]),
  ) as Record<YuneWebExport, EmscriptenWrappedFunction>;

  return {
    init: (sharedDataDir, userDataDir, schemaId) =>
      asNumber(wrapped.yune_web_init(sharedDataDir, userDataDir, schemaId)),
    processKey: (statePtr, keycode, mask) =>
      asNumber(wrapped.yune_web_process_key(statePtr, keycode, mask)),
    selectCandidate: (statePtr, index) =>
      asNumber(wrapped.yune_web_select_candidate(statePtr, index)),
    deleteCandidate: (statePtr, index) =>
      asNumber(wrapped.yune_web_delete_candidate(statePtr, index)),
    flipPage: (statePtr, backward) => asNumber(wrapped.yune_web_flip_page(statePtr, backward)),
    customize: (statePtr, configId, key, value) =>
      asNumber(wrapped.yune_web_customize(statePtr, configId, key, value)),
    setOption: (statePtr, option, value) =>
      asNumber(wrapped.yune_web_set_option(statePtr, option, value)),
    setAiEnabled: (statePtr, enabled) =>
      asNumber(wrapped.yune_web_set_ai_enabled(statePtr, enabled)),
    stageAi: (statePtr) => asNumber(wrapped.yune_web_stage_ai(statePtr)),
    deploy: (statePtr) => asNumber(wrapped.yune_web_deploy(statePtr)),
    cleanup: (statePtr) => {
      wrapped.yune_web_cleanup(statePtr);
    },
    responseJson: (responsePtr) => asNumber(wrapped.yune_web_response_json(responsePtr)),
    responseHandled: (responsePtr) => asNumber(wrapped.yune_web_response_handled(responsePtr)),
    freeResponse: (responsePtr) => {
      wrapped.yune_web_free_response(responsePtr);
    },
    module,
  };
}

function bindExport(
  module: EmscriptenYuneWebModule,
  symbol: YuneWebExport,
): EmscriptenWrappedFunction {
  const [returnType, argTypes] = SIGNATURES[symbol];
  const candidates = [symbol, LEGACY_EXPORTS[symbol]];
  const directExportVisible = candidates.some((candidate) => hasDirectExport(module, candidate));

  for (const candidate of candidates) {
    if (directExportVisible && !hasDirectExport(module, candidate)) {
      continue;
    }
    try {
      const wrapped = module.cwrap(candidate, returnType, [...argTypes]);
      if (typeof wrapped === "function") {
        return wrapped;
      }
    } catch {
      // Try the legacy generated WASM name below.
    }
  }
  throw new YuneWebBindingError(`Missing YuneWeb export: ${symbol}`);
}

function hasDirectExport(module: EmscriptenYuneWebModule, symbol: string): boolean {
  return typeof (module as unknown as Record<string, unknown>)[`_${symbol}`] === "function";
}

function asNumber(value: unknown): number {
  if (typeof value !== "number") {
    throw new YuneWebBindingError("YuneWeb export returned a non-number value");
  }
  return value;
}
