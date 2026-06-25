interface Scenario {
	label: string;
	input: string;
}

const SCENARIOS: Scenario[] = [
	{ label: "nei", input: "nei" },
	{ label: "ngo", input: "ngo" },
	{ label: "santai", input: "santai" },
	{ label: "m", input: "m" },
	{ label: "mgoi", input: "mgoi" },
	{ label: "tone letters", input: "seov" },
	{ label: "反查 `zhe", input: "`zhe" },
	{ label: "AI trigger", input: "nei" },
];

function keyCodeFor(char: string): string {
	if (/^[a-z]$/.test(char)) {
		return `Key${char.toUpperCase()}`;
	}
	return char;
}

function sendKeyboardEvent(textArea: HTMLTextAreaElement, type: "keydown" | "keyup", key: string, code = keyCodeFor(key)) {
	textArea.dispatchEvent(new KeyboardEvent(type, {
		key,
		code,
		bubbles: true,
		cancelable: true,
	}));
}

function sendPrintable(textArea: HTMLTextAreaElement, char: string) {
	sendKeyboardEvent(textArea, "keydown", char);
	sendKeyboardEvent(textArea, "keyup", char);
}

function resetInput(textArea: HTMLTextAreaElement) {
	textArea.focus();
	if (document.querySelector(".candidate-panel")) {
		sendKeyboardEvent(textArea, "keydown", "Escape", "Escape");
	}
	textArea.value = "";
	textArea.selectionStart = textArea.selectionEnd = 0;
	textArea.dispatchEvent(new Event("input", { bubbles: true }));
}

function delay(ms: number) {
	return new Promise(resolve => setTimeout(resolve, ms));
}

export default function YuneFeatureShowcase({ textArea }: { textArea: HTMLTextAreaElement }) {
	async function runScenario(input: string) {
		resetInput(textArea);
		await delay(120);
		for (const char of input) {
			sendPrintable(textArea, char);
			await delay(120);
		}
	}

	return <section className="my-4">
		<div className="flex flex-wrap gap-2">
			{SCENARIOS.map(scenario =>
				<button key={scenario.label} type="button" className="yd-button" onClick={() => void runScenario(scenario.input)}>
					{scenario.label}
				</button>
			)}
		</div>
	</section>;
}
