/// <reference types="vite/client" />

interface ImportMetaEnv {
	readonly VITE_YUNE_PUBLIC_DEMO?: string;
}

interface ImportMeta {
	readonly env: ImportMetaEnv;
}
