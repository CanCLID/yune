import type { ToastType } from "../toast";
import type { RimeResult } from "../types";

/**
 * Chooses the toast, if any, for one settled engine action in the candidate
 * panel (processKey, selectCandidate, deleteCandidate, flipPage, stageAi).
 *
 * A fulfilled result never toasts, even with `success: false`. That flag only
 * mirrors the engine's `handled == false`: the key was passed through, as RIME
 * does for Space or a digit with nothing composed, and the panel inserts the
 * raw character itself. Only a rejected action (an error thrown by the worker
 * or runtime) means the input method may be broken and warrants the reload
 * advice in the error toast.
 */
export function engineActionToastType(outcome: PromiseSettledResult<RimeResult>): ToastType | undefined {
	return outcome.status === "rejected" ? "error" : undefined;
}
