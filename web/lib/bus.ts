type EventKey = string | symbol;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type EventHandler<T = any> = (payload: T)=> void;
export type EventMap = Record<EventKey, EventHandler>;

export interface EventBus<T extends EventMap> {
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	on<Key extends keyof T>(key: Key, handler: T[Key], preParams?: any): ()=> void;
	off<Key extends keyof T>(key: Key, handlers: T[Key]): void;
	emit<Key extends keyof T>(key: Key, payload?: Parameters<T[Key]>[0]): void;
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	once<Key extends keyof T>(key: Key, handler: T[Key], preParams?: any): void;
}

type Bus<E> = Record<keyof E, E[keyof E][]>;

export function createEventBus<E extends EventMap>(config?: {
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	onError: (...params: any[])=> void;
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	preListen?<Key extends keyof E>(key: Key, handler: E[Key], preParams: any): boolean;
}): EventBus<E> {
	const bus: Partial<Bus<E>> = {};

	const off: EventBus<E>["off"] = (key, handler) => {
		const index = bus[key]?.indexOf(handler) ?? -1;
		// eslint-disable-next-line no-bitwise
		bus[key]?.splice(index >>> 0, 1);
	};

	const on: EventBus<E>["on"] = (key, handler, preParams) => {
		const intercept = config?.preListen?.(key, handler, preParams);
		if(intercept) return () => {};

		if(!bus[key]) bus[key] = [];

		bus[key]?.push(handler);

		return () => {
			off(key, handler);
		};
	};

	const emit: EventBus<E>["emit"] = (key, payload) => {
		bus[key]?.forEach((fn) => {
			try {
				fn(payload);
			} catch(e) {
				config?.onError(e);
			}
		});
	};

	const once: EventBus<E>["once"] = (key, handler, preParams) => {
		const intercept = config?.preListen?.(key, handler, preParams);
		if(intercept) return;

		const handleOnce = (payload: Parameters<typeof handler>) => {
			handler(payload);
			off(key, handleOnce as typeof handler);
		};

		on(key, handleOnce as typeof handler);
	};

	return {
		on,
		off,
		emit,
		once,
	};
}
