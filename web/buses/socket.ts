import { createEventBus, EventBus, EventMap } from "@/lib/bus";
import { SWAuthResponseData } from "@/packets/auth";
import { Event, ListenEvent } from "@/packets/events";
import { ID } from "@/packets/packet";

interface SocketBus extends EventMap {
	[ID.SWAuthResponse]: (packet: SWAuthResponseData)=> void;
	[ID.SWEvent]: (event: Event)=> void;
	[ID.WSListen]: (events: ListenEvent[])=> void;
	[ID.WSSync]: (daemonUuid: string)=> void;
	connected: ()=> void;
}

export const socketBus: EventBus<SocketBus> & { isConnected?: boolean; } = createEventBus<SocketBus>({
	onError: (e) => {
		console.error(e);
	},
	preListen: (key, handler) => {
		if(key === "connected" && socketBus.isConnected) {
			handler(null);
			return true;
		}

		return false;
	},
});

socketBus.isConnected = false;
socketBus.on("connected", () => {
	socketBus.isConnected = true;
});
