import { createEventBus, EventMap } from "@/lib/eventbus";
import { EventType, ID, NodeStatus } from "../types/packets";
import { socketBus } from "./socket";

interface EventsBus extends EventMap {
	[EventType.NodesList]: (payload: NodeStatus[])=> void;
}

export const eventsBus = createEventBus<EventsBus>({
	onError: (e) => {
		console.error(e);
	},
	preListen: (key) => {
		if(key === EventType.NodesList) {
			socketBus.once("connected", () => {
				socketBus.emit(ID.ASListen, {
					type: EventType.NodesList,
					data: [],
				});
			});
		}
	},
});
