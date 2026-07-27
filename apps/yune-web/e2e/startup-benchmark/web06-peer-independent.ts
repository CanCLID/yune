import type {
  ComparatorDomTuple,
  ComparatorStableObservation,
} from "./comparator-endpoint.ts";

// This intentionally duplicates the public tuple digest projection instead of
// importing its implementation. The Phase-4 verifier must detect a mutated
// tuple even when every supplied digest string was left internally equal.
export function independentComparatorDomTupleDigest(tuple: ComparatorDomTuple): string {
  return JSON.stringify({
    contractVersion: tuple.contractVersion,
    selectorManifestId: tuple.selectorManifestId,
    composition: tuple.composition,
    candidates: tuple.candidates.map(candidate => ({
      label: candidate.label,
      text: candidate.text,
      comment: candidate.comment,
    })),
    candidateSurfaceCount: tuple.candidateSurfaceCount,
    page: {
      index: tuple.page.index,
      buttonCount: tuple.page.buttonCount,
      previousDisabled: tuple.page.previousDisabled,
      nextDisabled: tuple.page.nextDisabled,
    },
    highlightedIndex: tuple.highlightedIndex,
    caret: {
      selectorCount: tuple.caret.selectorCount,
      value: tuple.caret.value,
      selectionStart: tuple.caret.selectionStart,
      selectionEnd: tuple.caret.selectionEnd,
      selectionDirection: tuple.caret.selectionDirection,
      active: tuple.caret.active,
      visible: tuple.caret.visible,
      disabled: tuple.caret.disabled,
    },
    status: {
      schemaId: tuple.status.schemaId,
      composing: tuple.status.composing,
      surfaceVisible: tuple.status.surfaceVisible,
      digest: tuple.status.digest,
    },
  });
}

export function independentStableObservationFailures(
  observation: ComparatorStableObservation | undefined,
): string[] {
  if (!observation) return ["observation-missing"];
  const failures: string[] = [];
  if (observation.event.deliveredAt === undefined
      || !Number.isFinite(observation.event.deliveredAt)
      || observation.event.deliveredAt < observation.event.timeStamp) {
    failures.push("event-delivery-clock");
  }
  if (observation.initial.revision <= observation.event.revisionBeforeEvent) {
    failures.push("revision-did-not-advance");
  }
  if (observation.initial.revision !== observation.firstRaf.revision
      || observation.firstRaf.revision !== observation.secondRaf.revision) {
    failures.push("revision-not-double-raf-stable");
  }
  for (const [name, tuple] of [
    ["initial", observation.initial],
    ["first-raf", observation.firstRaf],
    ["second-raf", observation.secondRaf],
  ] as const) {
    if (tuple.digest !== independentComparatorDomTupleDigest(tuple)) {
      failures.push(`${name}-digest-not-recomputed`);
    }
  }
  if (observation.initial.digest !== observation.firstRaf.digest
      || observation.firstRaf.digest !== observation.secondRaf.digest) {
    failures.push("digest-not-double-raf-stable");
  }
  if (observation.initial.observedAt < observation.event.timeStamp
      || observation.firstRaf.observedAt < observation.initial.observedAt
      || observation.secondRaf.observedAt < observation.firstRaf.observedAt) {
    failures.push("observation-clock-order");
  }
  return failures;
}
