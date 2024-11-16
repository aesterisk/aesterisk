import { createEventBus, EventMap } from "@/lib/eventbus";
import { Event, ID, SAAuthResponseData } from "../types/packets";

interface SocketBus extends EventMap {
	[ID.SAAuthResponse]: (packet: SAAuthResponseData)=> void;
	[ID.SAEvent]: (packet: Event)=> void;
	[ID.ASListen]: (packet: Event)=> void;
}

export const socketBus = createEventBus<SocketBus>({
	onError: (e) => {
		console.error(e);
	},
});
