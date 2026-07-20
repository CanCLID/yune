import type CandidateInfo from "./CandidateInfo";
import type { GrammarModelDiagnostic } from "./octagram";
import type { CandidateMenuLayout, ChineseTypefaceId, Language, OutputStandard, ShowRomanization } from "./consts";
import type { UiLanguage } from "./uiText";
import type { Dispatch, SetStateAction } from "react";
import type { YuneWebInspectorDebug, YuneWebStatus } from "@yune-ime/yune-web-runtime";

export type { GrammarModelDiagnostic } from "./octagram";

export interface RimeAPI {
	init(): boolean;
	set_option(option: string, value: number): void;
	process_key(input: string): string;
	select_candidate(index: number): string;
	delete_candidate(index: number): string;
	flip_page(backward: boolean): string;
	customize(page_size: number, options: number): boolean;
	deploy(): boolean;
}

export interface YuneWebUserdbRow {
	text: string;
	code: string;
	commits: number | null;
	dee: number | null;
	tick: number | null;
	raw: string;
}

export interface YuneWebUserdbParseError {
	line: number;
	raw: string;
	reason: string;
}

export interface YuneWebUserdbSnapshot {
	schemaId: RimeSchemaId;
	dictionaryId: string;
	path: string;
	exists: boolean;
	bytes: number;
	updatedAt: string | null;
	rows: YuneWebUserdbRow[];
	rawText: string;
	parseErrors: YuneWebUserdbParseError[];
}

export interface YuneWebMemorySnapshot {
	wasmHeapBytes: number;
	peakWasmHeapBytes: number;
}

export interface YuneDeployStampSnapshot {
	version?: number;
	assetVersion?: string;
	schemaId?: string;
	dictionaryId?: string;
	assetSignature?: string;
	customConfigSignature?: string;
	invalidatedAt?: string;
	reason?: string;
}

export interface YuneDeployCacheSnapshot {
	schemaId: string;
	dictionaryId: string;
	cacheFresh: boolean;
	deployedSchemaExists: boolean;
	actualStamp: YuneDeployStampSnapshot | null;
	expectedStamp: YuneDeployStampSnapshot;
}

export interface YuneInjectedAssetManifest {
	schemaId: RimeSchemaId;
	assets: {
		path: string;
		bytes: number;
		kind: "text" | "binary";
	}[];
}

export interface Actions {
	setOption(option: string, value: boolean): Promise<void>;
	selectSchema(schemaId: RimeSchemaId): Promise<boolean>;
	getUserdbSnapshot(): Promise<YuneWebUserdbSnapshot>;
	importUserdb(rawText: string): Promise<YuneWebUserdbSnapshot>;
	processKey(input: string): Promise<RimeResult>;
	stageAi(): Promise<RimeResult>;
	selectCandidate(index: number): Promise<RimeResult>;
	deleteCandidate(index: number): Promise<RimeResult>;
	flipPage(backward: boolean): Promise<RimeResult>;
	customize(preferences: Partial<RimePreferences>): Promise<boolean>;
	customizeValue(configId: string, key: string, value: string): Promise<boolean>;
	deploy(): Promise<boolean>;
	deployCacheSnapshot(): Promise<YuneDeployCacheSnapshot>;
	invalidateDeployCache(): Promise<YuneDeployCacheSnapshot>;
	injectedAssetsManifest(): Promise<YuneInjectedAssetManifest>;
}

interface InputBuffer {
	before: string;
	active: string;
	after: string;
}

interface RimeComposing {
	isComposing: true;
	inputBuffer: InputBuffer;
	page: number;
	isLastPage: boolean;
	highlightedIndex: number;
	candidates: {
		label?: string;
		text: string;
		comment?: string;
		source?: string;
		quality?: number;
		preedit?: string;
		aiConfidence?: number;
	}[];
	debug?: YuneWebInspectorDebug;
}

interface RimeNotComposing {
	isComposing: false;
}

interface RimePayload {
	success: boolean;
	committed?: string;
	status?: YuneWebStatus;
	memory?: YuneWebMemorySnapshot;
}

export type RimeResult = (RimeComposing | RimeNotComposing) & RimePayload;
export type YuneInspectorDebug = YuneWebInspectorDebug;
export type YuneStatusSnapshot = YuneWebStatus;
export type RimeSchemaId = "jyut6ping3" | "cangjie5" | "luna_pinyin" | "luna_pinyin_octagram";

export type RimeDeployStatus = "start" | "success" | "failure";

export interface RimeNotification {
	deploy: RimeDeployStatus;
	schema: `${string}/${string}`;
	option: string;
}

export interface ListenerArgsMap {
	deployStatusChanged: [status: RimeDeployStatus];
	schemaChanged: [id: string, name: string];
	grammarDiagnosticChanged: [diagnostic: GrammarModelDiagnostic];
	optionChanged: [option: string, value: boolean];
	initialized: [success: boolean, memory?: YuneWebMemorySnapshot];
}

interface NamedMessage<K extends keyof Actions> {
	name: K;
	args: Parameters<Actions[K]>;
	resolve: (value: ReturnType<Actions[K]>) => void;
	reject: (reason: unknown) => void;
	enqueuedAt?: number;
	sentAt?: number;
	web06: Web06ActionIdentity;
}

export type Message = NamedMessage<keyof Actions>;

/**
 * WEB-06 metadata is private to the yune-web main/worker transport. It never
 * enters the public Actions arguments or the runtime/native response payload.
 */
export const WEB06_PRIVATE_PROTOCOL_VERSION = "web06-private-v1" as const;

export type Web06CollectionMode = "off" | "minimal" | "full";
export type Web06ActionClass = "native-key" | "adapter-only" | "read-only" | "stateful-barrier";
export type Web06EventClassification = "mapped-action(s)" | "frontend-consumed" | "browser-pass-through";
export type Web06TerminalOutcome =
	| "painted"
	| "superseded"
	| "committed"
	| "processed-no-visual-change"
	| "barrier-completed"
	| "failure";

export type Web06BoundaryKind =
	| "none"
	| "correction"
	| "selection"
	| "paging"
	| "commit"
	| "modifier-release"
	| "cancel"
	| "focus-loss"
	| "schema"
	| "option"
	| "deploy"
	| "persistence"
	| "error";

export interface Web06DomEventSnapshot {
	type: "keydown" | "keyup" | "blur" | "click" | "change" | "mousedown" | "touchstart";
	code: string;
	key: string;
	timeStamp: number;
	repeat: boolean;
	ctrlKey: boolean;
	metaKey: boolean;
	altKey: boolean;
	shiftKey: boolean;
}

export interface Web06MappedAction {
	name: keyof Actions;
	args: unknown[];
	actionClass: Web06ActionClass;
	supersedable: boolean;
	boundary: Web06BoundaryKind;
	deferred?: boolean;
}

export interface Web06FanoutAction {
	owner: string;
	action: Web06MappedAction;
}

export interface Web06ControlEventLike {
	type: string;
	timeStamp: number;
	ctrlKey?: boolean;
	metaKey?: boolean;
	altKey?: boolean;
	shiftKey?: boolean;
	nativeEvent?: Web06ControlEventLike;
}

export interface Web06EventMapResult {
	classification: Web06EventClassification;
	reason: string;
	preventDefault: boolean;
	actions: Web06MappedAction[];
	shiftEffect?: "start" | "mark-chorded" | "finish-toggle" | "finish-without-toggle";
}

export interface Web06DomEventIdentity extends Web06DomEventSnapshot {
	protocolVersion: typeof WEB06_PRIVATE_PROTOCOL_VERSION;
	eventId: string;
	eventSequenceId: number;
	eventDeliveredAt: number;
	classification: Web06EventClassification;
	reason: string;
	mappedActionCount: number;
	compositionEpochId: number;
	supersessionSubRunId: number;
}

export interface Web06ActionIdentity {
	protocolVersion: typeof WEB06_PRIVATE_PROTOCOL_VERSION;
	actionId: string;
	sequenceId: number;
	eventId?: string;
	eventSequenceId?: number;
	eventActionIndex?: number;
	compositionEpochId: number;
	supersessionSubRunId: number;
	actionClass: Web06ActionClass;
	supersedable: boolean;
	boundary: Web06BoundaryKind;
	rawInputSequence: string[];
	originKind: "dom-event" | "background";
	originReason: string;
	causedByActionId?: string;
	causedBySequenceId?: number;
	causedByEventId?: string;
	causedByEventSequenceId?: number;
	actionEnqueuedAt: number;
	mainQueueDepthAtEnqueue: number;
	workerSentAt?: number;
	workerDispatchDepth?: number;
}

export interface Web06ActionContext {
	event?: Web06DomEventIdentity;
	eventActionIndex?: number;
	compositionEpochId: number;
	supersessionSubRunId: number;
	actionClass: Web06ActionClass;
	supersedable: boolean;
	boundary: Web06BoundaryKind;
	rawInputSequence: string[];
	originKind?: "dom-event" | "background";
	originReason?: string;
	causedByActionId?: string;
	causedBySequenceId?: number;
	causedByEventId?: string;
	causedByEventSequenceId?: number;
}

export interface Web06ClockPingEnvelope {
	protocolVersion: typeof WEB06_PRIVATE_PROTOCOL_VERSION;
	kind: "clock-ping";
	pingId: string;
	mainSentAt: number;
}

export interface Web06ClockEchoEnvelope {
	protocolVersion: typeof WEB06_PRIVATE_PROTOCOL_VERSION;
	kind: "clock-echo";
	pingId: string;
	mainSentAt: number;
	workerReceivedAt: number;
	workerSentAt: number;
}

export interface Web06WorkerActionEnvelope {
	protocolVersion: typeof WEB06_PRIVATE_PROTOCOL_VERSION;
	kind: "action";
	mode: Web06CollectionMode;
	identity: Web06ActionIdentity;
	name: keyof Actions;
	args: unknown[];
}

export interface Web06ComponentSpan {
	component: "runtime" | "adapter" | "persistence" | "collector";
	operation: string;
	stage: string;
	startedAt: number;
	finishedAt: number;
	outcome: "success" | "error";
}

export interface Web06WorkerReceipt {
	workerMessageReceivedAt: number;
	workerActionStartedAt: number;
	workerFinishedAt: number;
	runtimeSpans: Web06ComponentSpan[];
	adapterSpans: Web06ComponentSpan[];
	persistenceSpans: Web06ComponentSpan[];
	collectorSpans: Web06ComponentSpan[];
	engineRawJson?: string;
	observerFailures: string[];
}

export interface Web06WorkerResultEnvelope {
	protocolVersion: typeof WEB06_PRIVATE_PROTOCOL_VERSION;
	kind: "action-result";
	mode: Web06CollectionMode;
	identity: Web06ActionIdentity;
	resultType: "success" | "error";
	result?: ReturnType<Actions[keyof Actions]>;
	error?: unknown;
	receipt: Web06WorkerReceipt;
	elapsedMs?: number;
	workerStartedAt?: number;
	workerFinishedAt?: number;
	workerBaseElapsedMs?: number;
	workerAmplificationMs?: number;
	workerActionMultiplier?: number;
}

export type Web06WorkerRequestEnvelope = Web06ClockPingEnvelope | Web06WorkerActionEnvelope;
export type Web06WorkerResponseEnvelope = Web06ClockEchoEnvelope | Web06WorkerResultEnvelope;

export interface Web06ClockExchange {
	pingId: string;
	mainSentAt: number;
	workerReceivedAt: number;
	workerSentAt: number;
	mainReceivedAt: number;
	offset: number;
	netRtt: number;
	uncertainty: number;
}

export interface Web06PresentationFingerprint {
	input: string;
	page: number;
	isLastPage: boolean;
	highlightedIndex: number;
	candidates: {
		label: string;
		text: string;
		comment: string;
		source: string;
	}[];
	status: Record<string, unknown> | null;
	textareaValue: string;
	selectionStart: number;
	selectionEnd: number;
}

export interface Web06PresentationOutcomeReceipt {
	identity: Web06ActionIdentity;
	outcome: Web06TerminalOutcome;
	stateUpdateScheduledAt: number;
	stateCommittedAt?: number;
	firstRafAt?: number;
	terminalObservedAt: number;
	presentationExpected: Web06PresentationFingerprint;
	domObserved: Web06PresentationFingerprint;
	presentationDigest: string;
	supersededBySequenceId?: number;
	supersessionSequenceLag?: number;
}

export interface Web06ActionReceipt {
	identity: Web06ActionIdentity;
	returnedIdentity?: Web06ActionIdentity;
	name: keyof Actions;
	args: unknown[];
	mainResponseReceivedAt?: number;
	responseMappingStartedAt?: number;
	responseMappingFinishedAt?: number;
	worker?: Web06WorkerReceipt;
	presentation?: Web06PresentationOutcomeReceipt;
	resultType?: "success" | "error";
}

export interface Web06MeasurementInvalidation {
	code: string;
	detail: string;
	recordedAt: number;
	actionId?: string;
	eventId?: string;
}

export interface InputState {
	isPrevDisabled: boolean;
	isNextDisabled: boolean;
	page: number;
	isLastPage: boolean;
	inputBuffer: InputBuffer;
	candidates: CandidateInfo[];
	highlightedIndex: number;
}

export interface RimePreferences {
	pageSize: number;
	enableCompletion: boolean;
	enableCorrection: boolean;
	enableSentence: boolean;
	enableLearning: boolean;
	enableAI: boolean;
	combineCandidates: boolean;
	predictionNeverFirst: boolean;
	predictionThreshold: number;
	activeSchema: RimeSchemaId;
	isExtendedCharset: boolean;
	isDisabled: boolean;
	dictionaryExclude: string[];
	isAsciiMode: boolean;
	isFullShape: boolean;
	outputStandard: OutputStandard;
	isCangjie5: boolean;
}

export interface InterfacePreferences {
	uiLanguage: UiLanguage;
	displayLanguages: Set<Language>;
	mainLanguage: Language;
	chineseTypeface: ChineseTypefaceId;
	candidateMenuLayout: CandidateMenuLayout;
	isCandidatePanelFixed: boolean;
	showDictionaryByDefault: boolean;
	showRomanization: ShowRomanization;
	showReverseCode: boolean;
}

export type Preferences = RimePreferences & InterfacePreferences;

export type PreferencesWithSetter = Preferences & { [P in keyof Preferences as `set${Capitalize<P>}`]: Dispatch<SetStateAction<Preferences[P]>> };
