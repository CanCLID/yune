import type { Web06CollectionMode } from "../types";

export type Web06ProductConsoleLane = "product/no-probe" | Web06CollectionMode;

/**
 * WEB-06 collection mode must never suppress or add product console output.
 * These helpers retain the pre-instrumentation call shapes while accepting the
 * lane explicitly so owning tests can prove lane invariance.
 */
export function logWeb06PersistenceDiagnostic(
	_lane: Web06ProductConsoleLane,
	diagnostic: unknown,
): void {
	console.info(`YUNE_PERSISTENCE ${JSON.stringify(diagnostic)}`);
}

export function logWeb06RuntimeInitializationFailure(
	_lane: Web06ProductConsoleLane,
	error: unknown,
): void {
	console.error("Yune runtime initialization failed", error);
}

export function logWeb06NativeMessage(
	_lane: Web06ProductConsoleLane,
	message: string,
	publicDemo: boolean,
	search: string,
): void {
	if (publicDemo && search !== "?debug") return;
	const match = /^([IWEF])\S+ \S+ \S+ (.*)$/.exec(message);
	if (match) {
		console[({ I: "info", W: "warn", E: "error", F: "error" } as const)[match[1] as "I" | "W" | "E" | "F"]](`[${match[2]}`);
	}
	else {
		console.error(message);
	}
}

export function shouldLogWeb06ProductDebug(
	_lane: Web06ProductConsoleLane,
	isDev: boolean,
	search: string,
): boolean {
	return isDev || new URLSearchParams(search).has("debug");
}

export function logWeb06ActionError(
	_lane: Web06ProductConsoleLane,
	diagnostic: unknown,
): void {
	console.error("YUNE_WORKER_ACTION_ERROR", diagnostic);
}
