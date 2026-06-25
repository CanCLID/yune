import { keyEventToRimeKey, type YuneWebKeyboardEventLike } from "./keys.js";
import { bindYuneWebModule, type EmscriptenYuneWebModule, type YuneWebBindings } from "./module.js";
import { readYuneWebResponse, type YuneWebResponse } from "./response.js";

export interface YuneWebInitOptions {
  sharedDataDir: string;
  userDataDir: string;
  schemaId: string;
}

export class YuneWebLifecycleError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "YuneWebLifecycleError";
  }
}

export class YuneWebRuntime {
  #bindings: YuneWebBindings;
  #statePtr: number;
  #cleanedUp = false;

  private constructor(bindings: YuneWebBindings, statePtr: number) {
    this.#bindings = bindings;
    this.#statePtr = statePtr;
  }

  static init(module: EmscriptenYuneWebModule, options: YuneWebInitOptions): YuneWebRuntime {
    const bindings = bindYuneWebModule(module);
    const statePtr = bindings.init(options.sharedDataDir, options.userDataDir, options.schemaId);
    if (statePtr === 0) {
      throw new YuneWebLifecycleError("YuneWeb adapter init failed");
    }
    return new YuneWebRuntime(bindings, statePtr);
  }

  processKey(keycode: number, mask = 0): YuneWebResponse {
    const responsePtr = this.#bindings.processKey(this.requireLiveState(), keycode, mask);
    return readYuneWebResponse(responsePtr, this.#bindings);
  }

  processKeyboardEvent(event: YuneWebKeyboardEventLike): YuneWebResponse {
    const { keycode, mask } = keyEventToRimeKey(event);
    return this.processKey(keycode, mask);
  }

  selectCandidate(index: number): YuneWebResponse {
    const responsePtr = this.#bindings.selectCandidate(this.requireLiveState(), index);
    return readYuneWebResponse(responsePtr, this.#bindings);
  }

  deleteCandidate(index: number): YuneWebResponse {
    const responsePtr = this.#bindings.deleteCandidate(this.requireLiveState(), index);
    return readYuneWebResponse(responsePtr, this.#bindings);
  }

  flipPage(backward = false): YuneWebResponse {
    const responsePtr = this.#bindings.flipPage(this.requireLiveState(), backward ? 1 : 0);
    return readYuneWebResponse(responsePtr, this.#bindings);
  }

  deploy(): boolean {
    return this.#bindings.deploy(this.requireLiveState()) !== 0;
  }

  customize(configId: string, key: string, value: string): boolean {
    return this.#bindings.customize(this.requireLiveState(), configId, key, value) !== 0;
  }

  setOption(option: string, value: boolean): boolean {
    return this.#bindings.setOption(this.requireLiveState(), option, value ? 1 : 0) !== 0;
  }

  setAiEnabled(enabled: boolean): boolean {
    return this.#bindings.setAiEnabled(this.requireLiveState(), enabled ? 1 : 0) !== 0;
  }

  stageAi(): YuneWebResponse {
    const responsePtr = this.#bindings.stageAi(this.requireLiveState());
    return readYuneWebResponse(responsePtr, this.#bindings);
  }

  cleanup(): void {
    if (this.#cleanedUp) {
      return;
    }
    this.#cleanedUp = true;
    const ptr = this.#statePtr;
    this.#statePtr = 0;
    if (ptr !== 0) {
      this.#bindings.cleanup(ptr);
    }
  }

  private requireLiveState(): number {
    if (this.#cleanedUp || this.#statePtr === 0) {
      throw new YuneWebLifecycleError("YuneWeb runtime has been cleaned up");
    }
    return this.#statePtr;
  }
}

export type { YuneWebKeyboardEventLike };
