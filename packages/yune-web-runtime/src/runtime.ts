import { keyEventToRimeKey, type YuneWebKeyboardEventLike } from "./keys.js";
import { bindYuneWebModule, type EmscriptenYuneWebModule, type YuneWebBindings } from "./module.js";
import {
  activateWeb06RuntimeObservation,
  observeWeb06RuntimeStage,
  type Web06ActiveRuntimeObservation,
  type Web06RuntimeObservation,
  type Web06RuntimeOperation,
} from "./observation.js";
import { readYuneWebResponse, type YuneWebResponse } from "./response.js";

export interface YuneWebInitOptions {
  sharedDataDir: string;
  userDataDir: string;
  schemaId: string;
  /** @internal WEB-06 measurement-only observer. */
  web06Observation?: Web06RuntimeObservation;
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
  #web06Observation: Web06ActiveRuntimeObservation | undefined;
  #cleanedUp = false;

  private constructor(
    bindings: YuneWebBindings,
    statePtr: number,
    web06Observation: Web06ActiveRuntimeObservation | undefined,
  ) {
    this.#bindings = bindings;
    this.#statePtr = statePtr;
    this.#web06Observation = web06Observation;
  }

  static init(module: EmscriptenYuneWebModule, options: YuneWebInitOptions): YuneWebRuntime {
    const bindings = bindYuneWebModule(module);
    const web06Observation = activateWeb06RuntimeObservation(options.web06Observation);
    const statePtr = observeWeb06RuntimeStage(
      web06Observation,
      "init",
      "abi-call",
      "minimal",
      () => bindings.init(options.sharedDataDir, options.userDataDir, options.schemaId),
    );
    if (statePtr === 0) {
      throw new YuneWebLifecycleError("YuneWeb adapter init failed");
    }
    return new YuneWebRuntime(bindings, statePtr, web06Observation);
  }

  processKey(keycode: number, mask = 0): YuneWebResponse {
    const statePtr = this.requireLiveState();
    return this.readResponse("process-key", () =>
      this.#bindings.processKey(statePtr, keycode, mask),
    );
  }

  processKeyboardEvent(event: YuneWebKeyboardEventLike): YuneWebResponse {
    const { keycode, mask } = keyEventToRimeKey(event);
    return this.processKey(keycode, mask);
  }

  selectCandidate(index: number): YuneWebResponse {
    const statePtr = this.requireLiveState();
    return this.readResponse("select-candidate", () =>
      this.#bindings.selectCandidate(statePtr, index),
    );
  }

  deleteCandidate(index: number): YuneWebResponse {
    const statePtr = this.requireLiveState();
    return this.readResponse("delete-candidate", () =>
      this.#bindings.deleteCandidate(statePtr, index),
    );
  }

  flipPage(backward = false): YuneWebResponse {
    const statePtr = this.requireLiveState();
    return this.readResponse("flip-page", () =>
      this.#bindings.flipPage(statePtr, backward ? 1 : 0),
    );
  }

  deploy(): boolean {
    const statePtr = this.requireLiveState();
    return this.observeAbi("deploy", () => this.#bindings.deploy(statePtr)) !== 0;
  }

  customize(configId: string, key: string, value: string): boolean {
    const statePtr = this.requireLiveState();
    return this.observeAbi("customize", () =>
      this.#bindings.customize(statePtr, configId, key, value),
    ) !== 0;
  }

  setOption(option: string, value: boolean): boolean {
    const statePtr = this.requireLiveState();
    return this.observeAbi("set-option", () =>
      this.#bindings.setOption(statePtr, option, value ? 1 : 0),
    ) !== 0;
  }

  setAiEnabled(enabled: boolean): boolean {
    const statePtr = this.requireLiveState();
    return this.observeAbi("set-ai-enabled", () =>
      this.#bindings.setAiEnabled(statePtr, enabled ? 1 : 0),
    ) !== 0;
  }

  stageAi(): YuneWebResponse {
    const statePtr = this.requireLiveState();
    return this.readResponse("stage-ai", () => this.#bindings.stageAi(statePtr));
  }

  cleanup(): void {
    if (this.#cleanedUp) {
      return;
    }
    this.#cleanedUp = true;
    const ptr = this.#statePtr;
    this.#statePtr = 0;
    if (ptr !== 0) {
      this.observeAbi("cleanup", () => this.#bindings.cleanup(ptr));
    }
  }

  private observeAbi<T>(operation: Web06RuntimeOperation, action: () => T): T {
    return observeWeb06RuntimeStage(
      this.#web06Observation,
      operation,
      "abi-call",
      "minimal",
      action,
    );
  }

  private readResponse(
    operation: Web06RuntimeOperation,
    action: () => number,
  ): YuneWebResponse {
    const responsePtr = this.observeAbi(operation, action);
    return readYuneWebResponse(
      responsePtr,
      this.#bindings,
      operation,
      this.#web06Observation,
    );
  }

  private requireLiveState(): number {
    if (this.#cleanedUp || this.#statePtr === 0) {
      throw new YuneWebLifecycleError("YuneWeb runtime has been cleaned up");
    }
    return this.#statePtr;
  }
}

export type { YuneWebKeyboardEventLike };
