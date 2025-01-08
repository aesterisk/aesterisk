"use client";

import { eventsBus, EventsBus } from "@/buses/event";
import { useEffect } from "react";

export default function useEvent<Key extends keyof EventsBus>(event: Key, callback: EventsBus[Key], params?: unknown) {
	useEffect(() => {
		const unsubscribe = eventsBus.on(event, (payload) => {
			callback(payload);
		}, params);

		return unsubscribe;
	}, [event, callback, params]);
}
