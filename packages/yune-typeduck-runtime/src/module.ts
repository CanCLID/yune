export type EmscriptenCType = "number" | "string" | "boolean" | "array" | null;

export type EmscriptenWrappedFunction = (...args: unknown[]) => unknown;

export interface EmscriptenTypeDuckModule {
  cwrap(
    ident: string,
    returnType: EmscriptenCType,
    argTypes: EmscriptenCType[],
  ): EmscriptenWrappedFunction;
  UTF8ToString(ptr: number, maxBytesToRead?: number, ignoreNul?: boolean): string;
}

export const TYPEDUCK_EXPORTS = [
  "yune_typeduck_init",
  "yune_typeduck_process_key",
  "yune_typeduck_select_candidate",
  "yune_typeduck_delete_candidate",
  "yune_typeduck_flip_page",
  "yune_typeduck_deploy",
  "yune_typeduck_customize",
  "yune_typeduck_set_option",
  "yune_typeduck_set_ai_enabled",
  "yune_typeduck_stage_ai",
  "yune_typeduck_cleanup",
  "yune_typeduck_response_json",
  "yune_typeduck_response_handled",
  "yune_typeduck_free_response",
] as const;

export type TypeDuckExport = (typeof TYPEDUCK_EXPORTS)[number];

export class TypeDuckBindingError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "TypeDuckBindingError";
  }
}

export interface TypeDuckBindings {
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
  module: EmscriptenTypeDuckModule;
}

type Signature = readonly [returnType: EmscriptenCType, argTypes: readonly EmscriptenCType[]];

const SIGNATURES: Record<TypeDuckExport, Signature> = {
  yune_typeduck_init: ["number", ["string", "string", "string"]],
  yune_typeduck_process_key: ["number", ["number", "number", "number"]],
  yune_typeduck_select_candidate: ["number", ["number", "number"]],
  yune_typeduck_delete_candidate: ["number", ["number", "number"]],
  yune_typeduck_flip_page: ["number", ["number", "number"]],
  yune_typeduck_deploy: ["number", ["number"]],
  yune_typeduck_customize: ["number", ["number", "string", "string", "string"]],
  yune_typeduck_set_option: ["number", ["number", "string", "number"]],
  yune_typeduck_set_ai_enabled: ["number", ["number", "number"]],
  yune_typeduck_stage_ai: ["number", ["number"]],
  yune_typeduck_cleanup: [null, ["number"]],
  yune_typeduck_response_json: ["number", ["number"]],
  yune_typeduck_response_handled: ["number", ["number"]],
  yune_typeduck_free_response: [null, ["number"]],
};

export function bindTypeDuckModule(module: EmscriptenTypeDuckModule): TypeDuckBindings {
  const wrapped = Object.fromEntries(
    TYPEDUCK_EXPORTS.map((symbol) => [symbol, bindExport(module, symbol)]),
  ) as Record<TypeDuckExport, EmscriptenWrappedFunction>;

  return {
    init: (sharedDataDir, userDataDir, schemaId) =>
      asNumber(wrapped.yune_typeduck_init(sharedDataDir, userDataDir, schemaId)),
    processKey: (statePtr, keycode, mask) =>
      asNumber(wrapped.yune_typeduck_process_key(statePtr, keycode, mask)),
    selectCandidate: (statePtr, index) =>
      asNumber(wrapped.yune_typeduck_select_candidate(statePtr, index)),
    deleteCandidate: (statePtr, index) =>
      asNumber(wrapped.yune_typeduck_delete_candidate(statePtr, index)),
    flipPage: (statePtr, backward) => asNumber(wrapped.yune_typeduck_flip_page(statePtr, backward)),
    customize: (statePtr, configId, key, value) =>
      asNumber(wrapped.yune_typeduck_customize(statePtr, configId, key, value)),
    setOption: (statePtr, option, value) =>
      asNumber(wrapped.yune_typeduck_set_option(statePtr, option, value)),
    setAiEnabled: (statePtr, enabled) =>
      asNumber(wrapped.yune_typeduck_set_ai_enabled(statePtr, enabled)),
    stageAi: (statePtr) => asNumber(wrapped.yune_typeduck_stage_ai(statePtr)),
    deploy: (statePtr) => asNumber(wrapped.yune_typeduck_deploy(statePtr)),
    cleanup: (statePtr) => {
      wrapped.yune_typeduck_cleanup(statePtr);
    },
    responseJson: (responsePtr) => asNumber(wrapped.yune_typeduck_response_json(responsePtr)),
    responseHandled: (responsePtr) => asNumber(wrapped.yune_typeduck_response_handled(responsePtr)),
    freeResponse: (responsePtr) => {
      wrapped.yune_typeduck_free_response(responsePtr);
    },
    module,
  };
}

function bindExport(
  module: EmscriptenTypeDuckModule,
  symbol: TypeDuckExport,
): EmscriptenWrappedFunction {
  const [returnType, argTypes] = SIGNATURES[symbol];
  let wrapped: EmscriptenWrappedFunction;
  try {
    wrapped = module.cwrap(symbol, returnType, [...argTypes]);
  } catch {
    throw new TypeDuckBindingError(`Missing TypeDuck export: ${symbol}`);
  }
  if (typeof wrapped !== "function") {
    throw new TypeDuckBindingError(`Missing TypeDuck export: ${symbol}`);
  }
  return wrapped;
}

function asNumber(value: unknown): number {
  if (typeof value !== "number") {
    throw new TypeDuckBindingError("TypeDuck export returned a non-number value");
  }
  return value;
}
