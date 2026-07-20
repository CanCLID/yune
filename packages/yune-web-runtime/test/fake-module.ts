import type {
  EmscriptenCType,
  EmscriptenYuneWebModule,
  EmscriptenWrappedFunction,
  YuneWebExport,
} from "../src/module";
import { YUNE_WEB_EXPORTS } from "../src/module";

type CallMap = Record<string, unknown[][]>;
type FakeCallable = YuneWebExport | "UTF8ToString";

interface FakeResponse {
  jsonPtr: number;
  handled: boolean;
  freed: boolean;
}

export class FakeYuneWebModule implements EmscriptenYuneWebModule {
  #nextPtr = 1_000;
  #strings = new Map<number, string>();
  #responses = new Map<number, FakeResponse>();
  #exports = new Map<string, EmscriptenWrappedFunction>();
  #calls: CallMap = {};
  #callTrace: Array<{ symbol: string; args: unknown[] }> = [];
  #failures = new Map<FakeCallable, unknown>();

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
    this.record("UTF8ToString", [ptr]);
    this.throwIfConfigured("UTF8ToString");
    const value = this.#strings.get(ptr);
    if (value === undefined) {
      throw new Error(`Unexpected missing fake string pointer: ${ptr}`);
    }
    return value;
  }

  register(symbol: string, fn: EmscriptenWrappedFunction): void {
    this.#exports.set(symbol, fn);
  }

  remove(symbol: YuneWebExport): void {
    this.#exports.delete(symbol);
  }

  fail(symbol: FakeCallable, error: unknown): void {
    this.#failures.set(symbol, error);
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
    return this.calls("yune_web_free_response").map(([ptr]) => ptr as number);
  }

  calls(symbol: string): unknown[][] {
    return this.#calls[symbol] ?? [];
  }

  callTrace(): Array<{ symbol: string; args: unknown[] }> {
    return this.#callTrace.map(({ symbol, args }) => ({ symbol, args: [...args] }));
  }

  private registerDefaultExports(): void {
    for (const symbol of YUNE_WEB_EXPORTS) {
      this.#calls[symbol] = [];
    }

    this.register("yune_web_init", (...args) => {
      this.record("yune_web_init", args);
      this.throwIfConfigured("yune_web_init");
      return this.initResult;
    });
    this.register("yune_web_process_key", (...args) => {
      this.record("yune_web_process_key", args);
      this.throwIfConfigured("yune_web_process_key");
      return this.processKeyResult;
    });
    this.register("yune_web_select_candidate", (...args) => {
      this.record("yune_web_select_candidate", args);
      this.throwIfConfigured("yune_web_select_candidate");
      return this.selectCandidateResult;
    });
    this.register("yune_web_delete_candidate", (...args) => {
      this.record("yune_web_delete_candidate", args);
      this.throwIfConfigured("yune_web_delete_candidate");
      return this.deleteCandidateResult;
    });
    this.register("yune_web_flip_page", (...args) => {
      this.record("yune_web_flip_page", args);
      this.throwIfConfigured("yune_web_flip_page");
      return this.flipPageResult;
    });
    this.register("yune_web_deploy", (...args) => {
      this.record("yune_web_deploy", args);
      this.throwIfConfigured("yune_web_deploy");
      return this.deployResult;
    });
    this.register("yune_web_customize", (...args) => {
      this.record("yune_web_customize", args);
      this.throwIfConfigured("yune_web_customize");
      return this.customizeResult;
    });
    this.register("yune_web_set_option", (...args) => {
      this.record("yune_web_set_option", args);
      this.throwIfConfigured("yune_web_set_option");
      return this.setOptionResult;
    });
    this.register("yune_web_set_ai_enabled", (...args) => {
      this.record("yune_web_set_ai_enabled", args);
      this.throwIfConfigured("yune_web_set_ai_enabled");
      return this.setAiEnabledResult;
    });
    this.register("yune_web_stage_ai", (...args) => {
      this.record("yune_web_stage_ai", args);
      this.throwIfConfigured("yune_web_stage_ai");
      return this.stageAiResult;
    });
    this.register("yune_web_cleanup", (...args) => {
      this.record("yune_web_cleanup", args);
      this.throwIfConfigured("yune_web_cleanup");
    });
    this.register("yune_web_response_json", (...args) => {
      this.record("yune_web_response_json", args);
      this.throwIfConfigured("yune_web_response_json");
      const [ptr] = args as [number];
      return this.#responses.get(ptr)?.jsonPtr ?? 0;
    });
    this.register("yune_web_response_handled", (...args) => {
      this.record("yune_web_response_handled", args);
      this.throwIfConfigured("yune_web_response_handled");
      const [ptr] = args as [number];
      return this.#responses.get(ptr)?.handled === true ? 1 : 0;
    });
    this.register("yune_web_free_response", (...args) => {
      this.record("yune_web_free_response", args);
      this.throwIfConfigured("yune_web_free_response");
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
    this.#callTrace.push({ symbol, args: [...args] });
  }

  private throwIfConfigured(symbol: FakeCallable): void {
    if (this.#failures.has(symbol)) {
      throw this.#failures.get(symbol);
    }
  }
}
