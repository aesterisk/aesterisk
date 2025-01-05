"use client";

import { eventsBus, EventsBus } from "@/buses/event";
import { useEffect } from "react";

export default function useEvent<Key extends keyof EventsBus>(event: Key, callback: EventsBus[Key], params?: unknown) {
	useEffect(() => {
		console.log("MOUNT");

		const unsubscribe = eventsBus.on(event, (payload) => {
			callback(payload);
		}, params);

		return () => {
			console.log("UNMOUNT");
			unsubscribe();
		};
	}, [event, callback, params]);

	useEffect(() => {
		console.log("event MOUNT");

		return () => {
			console.log("event UNMOUNT");
		};
	}, [event]);

	useEffect(() => {
		console.log("callback MOUNT");

		return () => {
			console.log("callback UNMOUNT");
		};
	}, [callback]);

	useEffect(() => {
		console.log("params MOUNT");

		return () => {
			console.log("params UNMOUNT");
		};
	}, [params]);
}
