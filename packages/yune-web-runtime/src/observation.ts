/**
 * WEB-06 measurement-only runtime observation.
 *
 * This interface is private to the yune-web measurement lane. It must not be
 * used to change runtime results, native JSON, or response ownership.
 */
export type Web06RuntimeObservationMode = "off" | "minimal" | "full";

export type Web06RuntimeOperation =
  | "init"
  | "process-key"
  | "select-candidate"
  | "delete-candidate"
  | "flip-page"
  | "deploy"
  | "customize"
  | "set-option"
  | "set-ai-enabled"
  | "stage-ai"
  | "cleanup"
  | "direct-response-read";

export type Web06RuntimeStage =
  | "abi-call"
  | "response-json-accessor"
  | "response-byte-extraction"
  | "response-json-parse"
  | "response-shape-decode"
  | "response-handled-accessor"
  | "response-free";

export interface Web06RuntimeSpan {
  operation: Web06RuntimeOperation;
  stage: Web06RuntimeStage;
  startedAt: number;
  finishedAt: number;
  outcome: "returned" | "threw";
}

export interface Web06RuntimeResponseJsonCopy {
  operation: Web06RuntimeOperation;
  json: string;
}

export interface Web06RuntimeObservationFailure {
  operation: Web06RuntimeOperation;
  stage: Web06RuntimeStage;
  hook: "clock-start" | "clock-finish" | "span" | "response-json-copy";
  error: unknown;
}

export interface Web06RuntimeObservation {
  /** Captured once during runtime initialization and never read again. */
  mode: Web06RuntimeObservationMode;
  /** Monotonic clock in the worker realm. */
  now(): number;
  /**
   * Receives one completed component span synchronously between native/runtime
   * stages. The collector measures this callback's own entry/exit time with
   * the same clock; callback time is intentionally outside component spans.
   */
  onSpan(span: Web06RuntimeSpan): void;
  /**
   * Full mode only. Receives the exact string already returned by
   * UTF8ToString; the runtime neither re-reads nor clones native JSON for it.
   * The collector self-measures this callback in the same way as onSpan.
   */
  onResponseJsonCopy?(copy: Web06RuntimeResponseJsonCopy): void;
  /**
   * Out-of-band measurement failure sink. A binding evidence collector must
   * provide this and invalidate its run on every record.
   */
  onFailure?(failure: Web06RuntimeObservationFailure): void;
}

export interface Web06ActiveRuntimeObservation {
  readonly mode: Exclude<Web06RuntimeObservationMode, "off">;
  readonly now: () => number;
  readonly onSpan: (span: Web06RuntimeSpan) => void;
  readonly onResponseJsonCopy?: (copy: Web06RuntimeResponseJsonCopy) => void;
  readonly onFailure?: (failure: Web06RuntimeObservationFailure) => void;
}

export function activateWeb06RuntimeObservation(
  observation: Web06RuntimeObservation | undefined,
): Web06ActiveRuntimeObservation | undefined {
  if (observation === undefined || observation.mode === "off") {
    return undefined;
  }

  return {
    mode: observation.mode,
    now: observation.now.bind(observation),
    onSpan: observation.onSpan.bind(observation),
    ...(observation.onResponseJsonCopy === undefined
      ? {}
      : { onResponseJsonCopy: observation.onResponseJsonCopy.bind(observation) }),
    ...(observation.onFailure === undefined
      ? {}
      : { onFailure: observation.onFailure.bind(observation) }),
  };
}

export function observeWeb06RuntimeStage<T>(
  observation: Web06ActiveRuntimeObservation | undefined,
  operation: Web06RuntimeOperation,
  stage: Web06RuntimeStage,
  detail: "minimal" | "full",
  action: () => T,
): T {
  if (observation === undefined || (detail === "full" && observation.mode !== "full")) {
    return action();
  }

  const startedAt = readClock(observation, operation, stage, "clock-start");
  let outcome: Web06RuntimeSpan["outcome"] = "returned";
  try {
    return action();
  } catch (error) {
    outcome = "threw";
    throw error;
  } finally {
    if (startedAt !== undefined) {
      const finishedAt = readClock(observation, operation, stage, "clock-finish");
      if (finishedAt !== undefined) {
        if (finishedAt < startedAt) {
          reportFailure(observation, {
            operation,
            stage,
            hook: "clock-finish",
            error: new Error("WEB-06 runtime observation clock moved backwards"),
          });
        } else {
          emitSpan(observation, {
            operation,
            stage,
            startedAt,
            finishedAt,
            outcome,
          });
        }
      }
    }
  }
}

export function emitWeb06ResponseJsonCopy(
  observation: Web06ActiveRuntimeObservation | undefined,
  operation: Web06RuntimeOperation,
  json: string,
): void {
  if (observation?.mode !== "full" || observation.onResponseJsonCopy === undefined) {
    return;
  }

  try {
    // This passes the already-extracted string by reference; it does not copy
    // native bytes again. The callback is synchronous and intentionally sits
    // between component spans. Collectors measure callback self-time at their
    // own entry/exit with the same clock so runtime spans never recurse.
    observation.onResponseJsonCopy({ operation, json });
  } catch (error) {
    reportFailure(observation, {
      operation,
      stage: "response-byte-extraction",
      hook: "response-json-copy",
      error,
    });
  }
}

function readClock(
  observation: Web06ActiveRuntimeObservation,
  operation: Web06RuntimeOperation,
  stage: Web06RuntimeStage,
  hook: "clock-start" | "clock-finish",
): number | undefined {
  try {
    const value = observation.now();
    if (!Number.isFinite(value)) {
      throw new Error("WEB-06 runtime observation clock returned a non-finite value");
    }
    return value;
  } catch (error) {
    reportFailure(observation, { operation, stage, hook, error });
    return undefined;
  }
}

function emitSpan(
  observation: Web06ActiveRuntimeObservation,
  span: Web06RuntimeSpan,
): void {
  try {
    // Like the JSON-copy callback, onSpan is a synchronous self-measurement
    // seam. Its caller-owned timing stays outside component spans.
    observation.onSpan(span);
  } catch (error) {
    reportFailure(observation, {
      operation: span.operation,
      stage: span.stage,
      hook: "span",
      error,
    });
  }
}

function reportFailure(
  observation: Web06ActiveRuntimeObservation,
  failure: Web06RuntimeObservationFailure,
): void {
  try {
    observation.onFailure?.(failure);
  } catch {
    // Measurement failures are out-of-band and must not replace the runtime's
    // result or error. The owning collector is responsible for failing the
    // evidence lane when its failure callback itself is unavailable.
  }
}
