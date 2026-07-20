import {
	web06ControlAction,
	web06LiveOptionActions,
} from "./private-protocol";

import type { OutputStandard } from "../consts";
import type {
	RimePreferences,
	RimeSchemaId,
	Web06FanoutAction,
	Web06MappedAction,
} from "../types";

export const WEB06_ACTION_OWNER = {
	schema: "schema-effect",
	deployPreferences: "deploy-preferences-effect",
	liveOptions: "live-options-effect",
	aiSettings: "ai-settings-effect",
	inspector: "inspector-effect",
	userdb: "userdb-effect",
	control: "direct-control",
	longPressDelete: "candidate-long-press-delete",
} as const;

export interface Web06LiveOptionState {
	isAsciiMode: boolean;
	isFullShape: boolean;
	outputStandard: OutputStandard;
	activeSchema: RimeSchemaId;
	isExtendedCharset: boolean;
	isDisabled: boolean;
}

export type Web06DeployPreferenceSet = Pick<
	RimePreferences,
	| "pageSize"
	| "enableCompletion"
	| "enableCorrection"
	| "enableSentence"
	| "enableLearning"
	| "combineCandidates"
	| "predictionNeverFirst"
	| "predictionThreshold"
	| "dictionaryExclude"
	| "isCangjie5"
>;

export function web06LiveOptionFanout(state: Web06LiveOptionState): Web06FanoutAction[] {
	return web06LiveOptionActions({
		nextAsciiMode: state.isAsciiMode,
		isFullShape: state.isFullShape,
		outputStandard: state.outputStandard,
		activeSchema: state.activeSchema,
		isExtendedCharset: state.isExtendedCharset,
		isDisabled: state.isDisabled,
	}).map(action => owned(WEB06_ACTION_OWNER.liveOptions, action));
}

export function web06DeployPreferenceFanout(
	preferences: Web06DeployPreferenceSet,
): Web06FanoutAction[] {
	return [
		owned(
			WEB06_ACTION_OWNER.deployPreferences,
			web06ControlAction("customize", [preferences]),
		),
		owned(
			WEB06_ACTION_OWNER.deployPreferences,
			web06ControlAction("deploy", []),
		),
	];
}

export function web06SchemaChangeFanout(options: {
	nextSchema: RimeSchemaId;
	deployPreferences: Web06DeployPreferenceSet;
	liveOptions: Web06LiveOptionState;
	applyDeployPreferences: boolean;
}): Web06FanoutAction[] {
	const select = owned(
		WEB06_ACTION_OWNER.schema,
		web06ControlAction("selectSchema", [options.nextSchema]),
	);
	const live = web06LiveOptionFanout({
		...options.liveOptions,
		activeSchema: options.nextSchema,
	});
	if (!options.applyDeployPreferences) {
		return [select, ...live];
	}
	const deploy = web06DeployPreferenceFanout(options.deployPreferences);
	const firstLive = live[0];
	if (firstLive === undefined) {
		return [select, ...deploy];
	}
	// React runs the schema, deploy-preference, and live-option effects in source
	// order. Their first calls enqueue synchronously. The deploy continuation is
	// then queued behind the first live option before the remaining options.
	return [select, deploy[0]!, firstLive, deploy[1]!, ...live.slice(1)];
}

export function web06SingleActionFanout(
	owner: string,
	action: Web06MappedAction,
): Web06FanoutAction[] {
	return [owned(owner, action)];
}

function owned(owner: string, action: Web06MappedAction): Web06FanoutAction {
	return { owner, action };
}
