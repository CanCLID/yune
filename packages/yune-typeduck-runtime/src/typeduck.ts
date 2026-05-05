import { keyEventToRimeKey, type TypeDuckKeyboardEventLike } from "./keys.js";
import { bindTypeDuckModule, type EmscriptenTypeDuckModule, type TypeDuckBindings } from "./module.js";
import { readTypeDuckResponse, type TypeDuckResponse } from "./response.js";

export interface TypeDuckInitOptions {
  sharedDataDir: string;
  userDataDir: string;
  schemaId: string;
}

export class TypeDuckLifecycleError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "TypeDuckLifecycleError";
  }
}

export class TypeDuckRuntime {
  #bindings: TypeDuckBindings;
  #statePtr: number;
  #cleanedUp = false;

  private constructor(bindings: TypeDuckBindings, statePtr: number) {
    this.#bindings = bindings;
    this.#statePtr = statePtr;
  }

  static init(module: EmscriptenTypeDuckModule, options: TypeDuckInitOptions): TypeDuckRuntime {
    const bindings = bindTypeDuckModule(module);
    const statePtr = bindings.init(options.sharedDataDir, options.userDataDir, options.schemaId);
    if (statePtr === 0) {
      throw new TypeDuckLifecycleError("TypeDuck adapter init failed");
    }
    return new TypeDuckRuntime(bindings, statePtr);
  }

  processKey(keycode: number, mask = 0): TypeDuckResponse {
    const responsePtr = this.#bindings.processKey(this.requireLiveState(), keycode, mask);
    return readTypeDuckResponse(responsePtr, this.#bindings);
  }

  processKeyboardEvent(event: TypeDuckKeyboardEventLike): TypeDuckResponse {
    const { keycode, mask } = keyEventToRimeKey(event);
    return this.processKey(keycode, mask);
  }

  selectCandidate(index: number): TypeDuckResponse {
    const responsePtr = this.#bindings.selectCandidate(this.requireLiveState(), index);
    return readTypeDuckResponse(responsePtr, this.#bindings);
  }

  deleteCandidate(index: number): TypeDuckResponse {
    const responsePtr = this.#bindings.deleteCandidate(this.requireLiveState(), index);
    return readTypeDuckResponse(responsePtr, this.#bindings);
  }

  flipPage(backward = false): TypeDuckResponse {
    const responsePtr = this.#bindings.flipPage(this.requireLiveState(), backward ? 1 : 0);
    return readTypeDuckResponse(responsePtr, this.#bindings);
  }

  deploy(): boolean {
    return this.#bindings.deploy(this.requireLiveState()) !== 0;
  }

  customize(configId: string, key: string, value: string): boolean {
    return this.#bindings.customize(this.requireLiveState(), configId, key, value) !== 0;
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
      throw new TypeDuckLifecycleError("TypeDuck runtime has been cleaned up");
    }
    return this.#statePtr;
  }
}

export type { TypeDuckKeyboardEventLike };
