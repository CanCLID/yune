import { useEffect, useState } from "react";

export type ToastType = "info" | "success" | "warning" | "error" | "default";

interface ToastMessage {
	id: number;
	type: ToastType;
	body: string;
}

const listeners = new Set<(message: ToastMessage | undefined) => void>();
let currentMessage: ToastMessage | undefined;
let nextToastId = 1;

export function notify(type: ToastType, zh: string, en: string) {
	const body = `${zh}時發生錯誤。如輸入法不能正常運作，請重新載入頁面。\nAn error occurred while ${en}. If the input method does not work properly, please reload the page.`;
	publishToast({ id: nextToastId++, type, body });
}

export function ToastViewport() {
	const [message, setMessage] = useState(currentMessage);

	useEffect(() => {
		listeners.add(setMessage);
		return () => {
			listeners.delete(setMessage);
		};
	}, []);

	useEffect(() => {
		if (!message) {
			return;
		}
		const timeout = setTimeout(() => {
			if (currentMessage?.id === message.id) {
				publishToast(undefined);
			}
		}, 6200);
		return () => clearTimeout(timeout);
	}, [message]);

	if (!message) {
		return null;
	}

	return (
		<div className={`yd-toast yd-toast-${message.type}`} role="status" aria-live="polite">
			<button
				type="button"
				className="yd-toast-close"
				aria-label="Dismiss notification"
				onClick={() => publishToast(undefined)}>
				×
			</button>
			{message.body}
		</div>
	);
}

function publishToast(message: ToastMessage | undefined) {
	currentMessage = message;
	for (const listener of listeners) {
		listener(message);
	}
}
