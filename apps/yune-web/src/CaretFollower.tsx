import { forwardRef, useCallback, useLayoutEffect, useRef, useState } from "react";

import getCaretCoordinates from "textarea-caret";

import type { ComponentPropsWithRef, ForwardedRef } from "react";

function readLineHeight(textArea: HTMLTextAreaElement, fallbackHeight: number) {
	const styles = window.getComputedStyle(textArea);
	const lineHeight = Number.parseFloat(styles.lineHeight);
	if (Number.isFinite(lineHeight)) {
		return lineHeight;
	}

	const fontSize = Number.parseFloat(styles.fontSize);
	if (Number.isFinite(fontSize)) {
		return fontSize * 1.65;
	}

	return fallbackHeight || 24;
}

function assignForwardedRef(ref: ForwardedRef<HTMLDivElement>, value: HTMLDivElement | null) {
	if (typeof ref === "function") {
		ref(value);
	}
	else if (ref) {
		(ref as { current: HTMLDivElement | null }).current = value;
	}
}

const CaretFollower = forwardRef<HTMLDivElement, ComponentPropsWithRef<"div"> & { textArea: HTMLTextAreaElement }>(function CaretFollower({ textArea, children, ...rest }, ref) {
	const [position, setPosition] = useState({ x: 0, y: 0 });
	const element = useRef<HTMLDivElement | null>(null);
	const setElement = useCallback((value: HTMLDivElement | null) => {
		element.current = value;
		assignForwardedRef(ref, value);
	}, [ref]);
	useLayoutEffect(() => {
		function onSelectionChange() {
			if (document.activeElement === textArea) {
				const { top, left, height } = getCaretCoordinates(textArea, textArea.selectionStart);
				const origin = element.current?.offsetParent?.getBoundingClientRect();
				const textAreaRect = textArea.getBoundingClientRect();
				const lineHeight = readLineHeight(textArea, height);
				setPosition({
					x: textAreaRect.left - (origin?.left ?? 0) + left,
					y: textAreaRect.top - (origin?.top ?? 0) + top + Math.max(height, lineHeight) + 8 - textArea.scrollTop,
				});
			}
		}
		textArea.focus();
		onSelectionChange();
		document.addEventListener("selectionchange", onSelectionChange);
		window.addEventListener("resize", onSelectionChange);
		textArea.addEventListener("selectionchange", onSelectionChange);
		textArea.addEventListener("scroll", onSelectionChange);
		textArea.addEventListener("resize", onSelectionChange);
		return () => {
			document.removeEventListener("selectionchange", onSelectionChange);
			window.removeEventListener("resize", onSelectionChange);
			textArea.removeEventListener("selectionchange", onSelectionChange);
			textArea.removeEventListener("scroll", onSelectionChange);
			textArea.removeEventListener("resize", onSelectionChange);
		};
	}, [textArea]);
	return <div ref={setElement} style={{ left: position.x, top: position.y }} {...rest}>{children}</div>;
});

export default CaretFollower;
