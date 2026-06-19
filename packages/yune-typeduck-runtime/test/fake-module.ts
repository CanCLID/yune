import type {
  EmscriptenCType,
  EmscriptenTypeDuckModule,
  EmscriptenWrappedFunction,
  TypeDuckExport,
} from "../src/module";
import { TYPEDUCK_EXPORTS } from "../src/module";

type CallMap = Record<string, unknown[][]>;

interface FakeResponse {
  jsonPtr: number;
  handled: boolean;
  freed: boolean;
}

export class FakeTypeDuckModule implements EmscriptenTypeDuckModule {
  #nextPtr = 1_000;
  #strings = new Map<number, string>();
  #responses = new Map<number, FakeResponse>();
  #exports = new Map<string, EmscriptenWrappedFunction>();
  #calls: CallMap = {};

  initResult = 1;
  processKeyResult = 0;
  selectCandidateResult = 0;
  deleteCandidateResult = 0;
  flipPageResult = 0;
  deployResult = 1;
  customizeResult = 1;
  setOptionResult = 1;
  setAiEnabledResult = 1;
  stageAiResult = 0;

  constructor() {
    this.registerDefaultExports();
  }

  cwrap(
    ident: string,
    _returnType: EmscriptenCType,
    _argTypes: EmscriptenCType[],
  ): EmscriptenWrappedFunction {
    const wrapped = this.#exports.get(ident);
    if (wrapped === undefined) {
      throw new Error(`Unexpected missing fake export: ${ident}`);
    }
    return wrapped;
  }

  UTF8ToString(ptr: number): string {
    const value = this.#strings.get(ptr);
    if (value === undefined) {
      throw new Error(`Unexpected missing fake string pointer: ${ptr}`);
    }
    return value;
  }

  register(symbol: string, fn: EmscriptenWrappedFunction): void {
    this.#exports.set(symbol, fn);
  }

  remove(symbol: TypeDuckExport): void {
    this.#exports.delete(symbol);
  }

  response(json: unknown, handled = true): number {
    return this.responseWithJsonPointer(this.string(JSON.stringify(json)), handled);
  }

  responseText(jsonText: string, handled = true): number {
    return this.responseWithJsonPointer(this.string(jsonText), handled);
  }

  responseWithJsonPointer(jsonPtr: number, handled = true): number {
    const ptr = this.pointer();
    this.#responses.set(ptr, { jsonPtr, handled, freed: false });
    return ptr;
  }

  string(value: string): number {
    const ptr = this.pointer();
    this.#strings.set(ptr, value);
    return ptr;
  }

  freedResponses(): number[] {
    return this.calls("yune_typeduck_free_response").map(([ptr]) => ptr as number);
  }

  calls(symbol: string): unknown[][] {
    return this.#calls[symbol] ?? [];
  }

  private registerDefaultExports(): void {
    for (const symbol of TYPEDUCK_EXPORTS) {
      this.#calls[symbol] = [];
    }

    this.register("yune_typeduck_init", (...args) => {
      this.record("yune_typeduck_init", args);
      return this.initResult;
    });
    this.register("yune_typeduck_process_key", (...args) => {
      this.record("yune_typeduck_process_key", args);
      return this.processKeyResult;
    });
    this.register("yune_typeduck_select_candidate", (...args) => {
      this.record("yune_typeduck_select_candidate", args);
      return this.selectCandidateResult;
    });
    this.register("yune_typeduck_delete_candidate", (...args) => {
      this.record("yune_typeduck_delete_candidate", args);
      return this.deleteCandidateResult;
    });
    this.register("yune_typeduck_flip_page", (...args) => {
      this.record("yune_typeduck_flip_page", args);
      return this.flipPageResult;
    });
    this.register("yune_typeduck_deploy", (...args) => {
      this.record("yune_typeduck_deploy", args);
      return this.deployResult;
    });
    this.register("yune_typeduck_customize", (...args) => {
      this.record("yune_typeduck_customize", args);
      return this.customizeResult;
    });
    this.register("yune_typeduck_set_option", (...args) => {
      this.record("yune_typeduck_set_option", args);
      return this.setOptionResult;
    });
    this.register("yune_typeduck_set_ai_enabled", (...args) => {
      this.record("yune_typeduck_set_ai_enabled", args);
      return this.setAiEnabledResult;
    });
    this.register("yune_typeduck_stage_ai", (...args) => {
      this.record("yune_typeduck_stage_ai", args);
      return this.stageAiResult;
    });
    this.register("yune_typeduck_cleanup", (...args) => {
      this.record("yune_typeduck_cleanup", args);
    });
    this.register("yune_typeduck_response_json", (...args) => {
      this.record("yune_typeduck_response_json", args);
      const [ptr] = args as [number];
      return this.#responses.get(ptr)?.jsonPtr ?? 0;
    });
    this.register("yune_typeduck_response_handled", (...args) => {
      this.record("yune_typeduck_response_handled", args);
      const [ptr] = args as [number];
      return this.#responses.get(ptr)?.handled === true ? 1 : 0;
    });
    this.register("yune_typeduck_free_response", (...args) => {
      this.record("yune_typeduck_free_response", args);
      const [ptr] = args as [number];
      const response = this.#responses.get(ptr);
      if (response !== undefined) {
        response.freed = true;
      }
    });
  }

  private pointer(): number {
    const ptr = this.#nextPtr;
    this.#nextPtr += 1;
    return ptr;
  }

  private record(symbol: string, args: unknown[]): void {
    (this.#calls[symbol] ??= []).push(args);
  }
}
