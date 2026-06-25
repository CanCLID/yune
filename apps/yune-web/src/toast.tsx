import { useEffect, useState } from "react";

import { DEFAULT_UI_LANGUAGE } from "./uiText";

import type { UiLanguage } from "./uiText";

export type ToastType = "info" | "success" | "warning" | "error" | "default";

interface ToastMessage {
	id: number;
	type: ToastType;
	enBody: string;
	yueBody: string;
}

const listeners = new Set<(message: ToastMessage | undefined) => void>();
let currentMessage: ToastMessage | undefined;
let nextToastId = 1;
let currentUiLanguage: UiLanguage = DEFAULT_UI_LANGUAGE;

export function setToastLanguage(uiLanguage: UiLanguage) {
	currentUiLanguage = uiLanguage;
	if (currentMessage) {
		publishToast({ ...currentMessage });
	}
}

export function notify(type: ToastType, zh: string, en: string) {
	publishToast({
		id: nextToastId++,
		type,
		yueBody: `${zh}時發生錯誤。如輸入法不能正常運作，請重新載入頁面。`,
		enBody: `An error occurred while ${en}. If the input method does not work properly, please reload the page.`,
	});
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
				aria-label={currentUiLanguage === "yue" ? "關閉通知" : "Dismiss notification"}
				onClick={() => publishToast(undefined)}>
				×
			</button>
			{currentUiLanguage === "yue" ? message.yueBody : message.enBody}
		</div>
	);
}

function publishToast(message: ToastMessage | undefined) {
	currentMessage = message;
	for (const listener of listeners) {
		listener(message);
	}
}
